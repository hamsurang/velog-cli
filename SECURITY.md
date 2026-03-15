# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| latest  | :white_check_mark: |
| < latest | :x:               |

## Reporting a Vulnerability

**Please do NOT open a public issue for security vulnerabilities.**

Use [GitHub Private Vulnerability Reporting](https://github.com/hamsurang/velog-cli/security/advisories/new) to report security issues.

### What to include

- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

### Response Timeline

- **Acknowledgment**: Within 48 hours
- **Initial assessment**: Within 7 days
- **Fix release**: As soon as practical

## Scope

- Token storage and handling (`~/.config/velog-cli/credentials.json`)
- Network communication with velog.io API
- Authentication flow (JWT validation, token refresh)
- Command injection via user inputs

## Out of Scope

- Vulnerabilities in velog.io itself (report to velog.io directly)
- Issues requiring physical access to the user's machine
