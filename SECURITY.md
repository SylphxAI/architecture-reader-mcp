# Security

Architecture Reader MCP is a local-first Beta 0.1 MCP package. It is not published
to npm or listed in public MCP directories yet.

## Reporting

Report suspected vulnerabilities privately through the SylphxAI organization
security process for this repository. Do not open public issues for sensitive
security reports.

## Current Security Boundary

- The MCP adapter uses stdio transport only; it does not expose HTTP endpoints.
- The engine does not store credentials.
- Indexing does not make network calls during repository analysis unless the
  caller explicitly configures remote behavior.
- Future tool responses must not expose file contents beyond the repository root
  being indexed.

## Required Before Release

- Access-control design for any HTTP transport.
- Path traversal and symlink boundary tests.
- Secret redaction tests for evidence snippets.
- Dependency audit and CI gate.
