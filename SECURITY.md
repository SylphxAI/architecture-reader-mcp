# Security

Architecture Reader MCP is currently a draft local-first MCP repository. It is
not released or deployed.

## Reporting

Report suspected vulnerabilities privately through the SylphxAI organization
security process for this repository. Do not open public issues for sensitive
security reports.

## Current Security Boundary

- The scaffold does not implement network transport beyond planned stdio MCP
  metadata.
- The scaffold does not store credentials.
- Future indexing code must not make network calls during repository analysis
  unless the user explicitly configures that behavior.
- Future tool responses must not expose file contents beyond the repository root
  being indexed.

## Required Before Release

- Access-control design for any HTTP transport.
- Path traversal and symlink boundary tests.
- Secret redaction tests for evidence snippets.
- Dependency audit and CI gate.
