# Ground Truth

## External Facts

- PR title/body: fix Host header so user information is not included.
- Body cites RFC 9110 section 7.2 and RFC 3986 section 3.2.
- Comments discuss keeping examples simple and compatibility around host/port formatting.
- Diff updates examples and server/client connection code; it also changes HTTP/2 default max header list size.

## True Risks

- severity: `high`
- expected reviewer concern: Host header must not leak URI username/password and must remain compatible with server routing.
- risk mechanism: using URI authority can include userinfo; changing host/port construction affects protocol-visible headers.

## Non-Risks

- Generic moved unwrap/panic-like code is not the main risk unless tied to new error behavior.

## Expected Evidence

- useful: protocol/security Host header construction change, examples changing from authority to host/port, HTTP/2 default size behavior change if intentional.
- misleading: generic blast radius without naming Host/security/protocol.
