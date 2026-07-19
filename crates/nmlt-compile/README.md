# `nmlt-compile`

`nmlt-compile` is the integrated M9 source-to-checked-program driver. It sends
exact source modules through the lossless surface projection, closed resolver,
bidirectional elaborator, neutral raw-certificate boundary, and independent
kernel. It returns only kernel-issued `CheckedProgram` values.

The driver is orchestration, not a second parser or an acceptance authority.
Projection, resolution, elaboration, and kernel failures retain distinct
stable diagnostic classes. Multi-module callers must provide the complete
closed source set and portable repository-relative paths.
