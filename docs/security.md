# security.md

Core practices for keeping this API and its data safe.

## Practices

| Practice                                       | What to check                                                                                                            | Why it matters                                                                                     |
| ---------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------- |
| Validate all inputs                            | Treat all external input as untrusted. Validate type, length, format, and range. Use allow-lists where possible.         | Prevents common attacks such as SQL injection, command injection, and malformed input abuse.       |
| Encode output by context                       | Encode data before rendering it in HTML, JavaScript, URLs, or other output contexts.                                     | Reduces the risk of cross-site scripting (XSS) and output-based injection flaws.                   |
| Use strong authentication and session controls | Enforce strong password handling, MFA where appropriate, secure session tokens, logout invalidation, and session expiry. | Helps prevent unauthorized access, session hijacking, credential abuse, and account compromise.    |
| Apply least privilege                          | Give users, services, containers, and application components only the permissions they need.                             | Limits blast radius if a component, credential, or account is compromised.                         |
| Enforce secure access control                  | Verify authorization on every sensitive action and object access. Do not rely on UI controls alone.                      | Prevents broken access control, privilege escalation, and insecure direct object reference issues. |
| Handle errors securely                         | Show generic errors to users and keep detailed diagnostics in protected logs only.                                       | Prevents information leakage that helps attackers map the system or exploit weaknesses.            |
| Protect secrets and sensitive data             | Never hardcode secrets in code, config files, or scripts. Use environment variables.                                     | Helps prevent credential leaks, unauthorized access, and downstream supply chain exposure.         |
| Log security-relevant events                   | Record failed logins, privilege changes, and sensitive actions without logging secrets or personal data.                 | Improves detection, investigation, and response while reducing exposure of sensitive information.  |
| Use proven cryptography                        | Use standard libraries and approved algorithms. Protect keys properly. Encrypt sensitive data in transit and at rest.    | Prevents weak encryption, key exposure, and unsafe custom implementations.                         |
