# Delta: ttsd-protocol

## MODIFIED Requirements

### Requirement: Shutdown Command

The `shutdown` request (`{ "cmd": "shutdown" }`) SHALL make ttsd respond
`{ "ok": true }` and then exit with code 0. On the Rust side, after sending
`shutdown` the driver closes stdin and waits up to 5 seconds for the process
to exit. If it does not, the driver SHALL escalate in stages: first send
SIGTERM, wait up to 2 more seconds for a clean exit, and only then force-kill
the still-running process (SIGKILL via `start_kill`).

#### Scenario: graceful shutdown

- GIVEN a running ttsd
- WHEN the Rust side sends `{"cmd":"shutdown"}`
- THEN ttsd writes `{"ok":true}` and terminates with exit code 0

#### Scenario: unresponsive shutdown is force-killed

- GIVEN a ttsd that does not exit after the shutdown response
- WHEN 5 seconds elapse
- THEN the Rust driver sends SIGTERM and force-kills the process only if it
  is still alive 2 seconds later
