package dev.aiterminal.android

object TermuxHelperBootstrapContract {
    const val HELPER_HOME = "\$HOME/.ash-termux-bridge"
    const val HELPER_PATH = "$HELPER_HOME/helper.sh"
    const val SELF_TEST_MARKER = "ASH_TERMUX_HELPER_OK"

    fun installScript(): String =
        """
        set -eu
        mkdir -p "$HELPER_HOME"
        cat > "$HELPER_PATH" <<'ASH_TERMUX_HELPER'
        #!/data/data/com.termux/files/usr/bin/sh
        set -u
        TERMUX_PREFIX=/data/data/com.termux/files/usr
        PATH="${'$'}TERMUX_PREFIX/bin:${'$'}PATH"
        export PATH

        helper_usage() {
          echo "usage: helper.sh self-test|run <job-dir>" >&2
          exit 64
        }

        helper_require_python() {
          if command -v python3 >/dev/null 2>&1; then
            return 0
          fi
          return 1
        }

        helper_json_escape() {
          sed 's/\\/\\\\/g; s/"/\\"/g; s/\r/\\r/g'
        }

        helper_emit() {
          helper_seq="${'$'}((helper_seq + 1))"
          helper_type="${'$'}1"
          helper_text="${'$'}2"
          helper_escaped="$(printf '%s' "${'$'}helper_text" | helper_json_escape)"
          printf '{"seq":%s,"type":"%s","text":"%s\\n"}\n' "${'$'}helper_seq" "${'$'}helper_type" "${'$'}helper_escaped" >> "${'$'}EVENTS_FILE"
        }

        helper_emit_final() {
          helper_seq="${'$'}((helper_seq + 1))"
          printf '{"seq":%s,"type":"finished","exit_code":%s}\n' "${'$'}helper_seq" "${'$'}1" >> "${'$'}EVENTS_FILE"
        }

        helper_emit_cancelled() {
          helper_seq="${'$'}((helper_seq + 1))"
          printf '{"seq":%s,"type":"cancelled","reason":"user"}\n' "${'$'}helper_seq" >> "${'$'}EVENTS_FILE"
        }

        helper_pump_log() {
          pump_type="${'$'}1"
          pump_file="${'$'}2"
          pump_seen="${'$'}3"
          if [ ! -f "${'$'}pump_file" ]; then
            printf '%s\n' "${'$'}pump_seen"
            return 0
          fi
          pump_total="$(wc -l < "${'$'}pump_file" | tr -d ' ')"
          if [ -z "${'$'}pump_total" ]; then
            pump_total=0
          fi
          if [ "${'$'}pump_total" -le "${'$'}pump_seen" ]; then
            printf '%s\n' "${'$'}pump_seen"
            return 0
          fi
          seq_file="${'$'}JOB_DIR/.ash-helper-seq"
          tail -n +"${'$'}((pump_seen + 1))" "${'$'}pump_file" | awk \
            -v seq="${'$'}helper_seq" \
            -v event_type="${'$'}pump_type" \
            -v seq_file="${'$'}seq_file" \
            'function esc(value) { gsub(/\\/, "\\\\", value); gsub(/"/, "\\\"", value); return value }
             { seq += 1; printf "{\"seq\":%d,\"type\":\"%s\",\"text\":\"%s\\n\"}\n", seq, event_type, esc($0) }
             END { print seq > seq_file }' >> "${'$'}EVENTS_FILE"
          if [ -f "${'$'}seq_file" ]; then
            helper_seq="$(cat "${'$'}seq_file")"
            rm -f "${'$'}seq_file"
          fi
          printf '%s\n' "${'$'}pump_total"
        }

        helper_run_shell_fallback() {
          ARGV_DIR="${'$'}JOB_DIR/argv"
          if [ ! -d "${'$'}ARGV_DIR" ]; then
            helper_emit stderr "python3 required and argv fallback missing"
            helper_emit_final 86
            exit 86
          fi

          set --
          found_argv=0
          for argv_file in "${'$'}ARGV_DIR"/*; do
            if [ ! -f "${'$'}argv_file" ]; then
              continue
            fi
            found_argv=1
            set -- "${'$'}@" "$(cat "${'$'}argv_file")"
          done
          if [ "${'$'}found_argv" -ne 1 ]; then
            helper_emit stderr "invalid argv fallback"
            helper_emit_final 64
            exit 64
          fi

          CWD="${'$'}JOB_DIR"
          if [ -f "${'$'}JOB_DIR/cwd" ]; then
            CWD="$(cat "${'$'}JOB_DIR/cwd")"
          fi
          if [ ! -d "${'$'}CWD" ]; then
            CWD="${'$'}JOB_DIR"
          fi

          STDOUT_LOG="${'$'}JOB_DIR/stdout.log"
          STDERR_LOG="${'$'}JOB_DIR/stderr.log"
          : > "${'$'}STDOUT_LOG"
          : > "${'$'}STDERR_LOG"
          stdout_seen=0
          stderr_seen=0

          ( cd "${'$'}CWD" && "${'$'}@" > "${'$'}STDOUT_LOG" 2> "${'$'}STDERR_LOG" ) &
          child="${'$'}!"
          helper_seq="${'$'}((helper_seq + 1))"
          printf '{"seq":%s,"type":"started","pid":%s}\n' "${'$'}helper_seq" "${'$'}child" >> "${'$'}EVENTS_FILE"

          cancelled=0
          while kill -0 "${'$'}child" >/dev/null 2>&1; do
            stdout_seen="$(helper_pump_log stdout "${'$'}STDOUT_LOG" "${'$'}stdout_seen")"
            stderr_seen="$(helper_pump_log stderr "${'$'}STDERR_LOG" "${'$'}stderr_seen")"
            if [ -f "${'$'}JOB_DIR/cancel" ]; then
              cancelled=1
              kill -INT "${'$'}child" >/dev/null 2>&1 || true
              sleep 2
              if kill -0 "${'$'}child" >/dev/null 2>&1; then
                kill -TERM "${'$'}child" >/dev/null 2>&1 || true
              fi
              sleep 1
              if kill -0 "${'$'}child" >/dev/null 2>&1; then
                kill -KILL "${'$'}child" >/dev/null 2>&1 || true
              fi
              break
            fi
            sleep 0.1
          done

          wait "${'$'}child"
          exit_code="${'$'}?"
          stdout_seen="$(helper_pump_log stdout "${'$'}STDOUT_LOG" "${'$'}stdout_seen")"
          stderr_seen="$(helper_pump_log stderr "${'$'}STDERR_LOG" "${'$'}stderr_seen")"

          if [ "${'$'}cancelled" -eq 1 ]; then
            helper_emit_cancelled
          else
            helper_emit_final "${'$'}exit_code"
          fi
          exit 0
        }

        if [ "${'$'}{1:-}" = "self-test" ]; then
          if ! helper_require_python; then
            echo "python3 missing; shell fallback enabled"
          fi
          echo "$SELF_TEST_MARKER"
          exit 0
        fi

        if [ "${'$'}{1:-}" != "run" ]; then
          helper_usage
        fi

        JOB_DIR="${'$'}{2:-}"
        if [ -z "${'$'}JOB_DIR" ]; then
          helper_usage
        fi

        if ! mkdir -p "${'$'}JOB_DIR"; then
          echo "Termux bridge job dir not writable: ${'$'}JOB_DIR" >&2
          exit 73
        fi
        EVENTS_FILE="${'$'}JOB_DIR/events.ndjson"
        if ! helper_require_python; then
          helper_seq=0
          helper_run_shell_fallback
        fi

        export ASH_TERMUX_JOB_DIR="${'$'}JOB_DIR"
        exec python3 - <<'ASH_TERMUX_PY'
        import json
        import os
        import signal
        import subprocess
        import sys
        import threading
        import time

        job_dir = os.environ["ASH_TERMUX_JOB_DIR"]
        request_path = os.path.join(job_dir, "request.json")
        events_path = os.path.join(job_dir, "events.ndjson")
        cancel_path = os.path.join(job_dir, "cancel")
        seq = 0
        event_lock = threading.Lock()

        def emit(payload):
            global seq
            with event_lock:
                seq += 1
                payload = dict(payload)
                payload["seq"] = seq
                with open(events_path, "a", encoding="utf-8") as events:
                    events.write(json.dumps(payload, separators=(",", ":")) + "\n")
                    events.flush()

        def finish_error(message, exit_code=1):
            emit({"type": "stderr", "text": message + "\n"})
            emit({"type": "finished", "exit_code": exit_code})
            return exit_code

        try:
            with open(request_path, "r", encoding="utf-8") as request_file:
                request = json.load(request_file)
            argv = request.get("argv") or []
            if not isinstance(argv, list) or not argv or not all(isinstance(arg, str) for arg in argv):
                sys.exit(finish_error("invalid argv in request.json", 64))
            cwd = request.get("cwd") or os.getcwd()
            if not os.path.isdir(cwd):
                cwd = os.getcwd()
            env = os.environ.copy()
            extra_env = request.get("env") or {}
            if isinstance(extra_env, dict):
                env.update({str(key): str(value) for key, value in extra_env.items()})
        except Exception as error:
            sys.exit(finish_error("failed to read request.json: " + str(error), 65))

        try:
            process = subprocess.Popen(
                argv,
                cwd=cwd,
                env=env,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                start_new_session=True,
            )
        except FileNotFoundError:
            sys.exit(finish_error("command not found: " + argv[0], 127))
        except Exception as error:
            sys.exit(finish_error("failed to start command: " + str(error), 126))

        emit({"type": "started", "pid": process.pid})
        cancelled = threading.Event()

        def pump(pipe, event_type):
            while True:
                data = pipe.readline()
                if not data:
                    break
                emit({"type": event_type, "text": data.decode("utf-8", "replace")})

        def watch_cancel():
            while process.poll() is None:
                if os.path.exists(cancel_path):
                    cancelled.set()
                    try:
                        os.killpg(process.pid, signal.SIGINT)
                    except ProcessLookupError:
                        return
                    deadline = time.time() + 2.0
                    while process.poll() is None and time.time() < deadline:
                        time.sleep(0.05)
                    if process.poll() is None:
                        try:
                            os.killpg(process.pid, signal.SIGTERM)
                        except ProcessLookupError:
                            return
                    deadline = time.time() + 1.0
                    while process.poll() is None and time.time() < deadline:
                        time.sleep(0.05)
                    if process.poll() is None:
                        try:
                            os.killpg(process.pid, signal.SIGKILL)
                        except ProcessLookupError:
                            return
                    return
                time.sleep(0.1)

        threads = [
            threading.Thread(target=pump, args=(process.stdout, "stdout"), daemon=True),
            threading.Thread(target=pump, args=(process.stderr, "stderr"), daemon=True),
            threading.Thread(target=watch_cancel, daemon=True),
        ]
        for thread in threads:
            thread.start()

        exit_code = process.wait()
        for thread in threads[:2]:
            thread.join(timeout=1.0)

        if cancelled.is_set():
            emit({"type": "cancelled", "reason": "user"})
        else:
            emit({"type": "finished", "exit_code": exit_code})
        ASH_TERMUX_PY
        ASH_TERMUX_HELPER
        chmod 700 "$HELPER_PATH"
        "$HELPER_PATH" self-test
        """.trimIndent()
}
