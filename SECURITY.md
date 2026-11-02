# Security policy

## Supported versions

Shelf is pre-1.0. Security fixes land on `main`; there are no maintained
release branches yet.

## Reporting

Please do not open a public issue for a vulnerability. Report it privately to
the maintainer, including:

- affected version or commit;
- reproduction steps;
- impact and any known workaround.

You should receive an acknowledgement within a few days. Please allow time for
a fix before public disclosure.

## Scope

Shelf reads local files, writes its own SQLite index and thumbnail cache, and
moves files through the freedesktop Trash specification. It does not upload
file contents or metadata.

Relevant areas include path traversal through IPC/protocol handlers, unsafe
external-process invocation, Trash path validation, symlink handling, xattr
parsing and the `thumb://` protocol.
