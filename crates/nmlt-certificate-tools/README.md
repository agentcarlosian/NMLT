# nmlt-certificate-tools

Untrusted transformations and measurements for raw NMLT elaboration
certificates. The crate can remove records that are unreachable from required
roots, restore canonical ordering, and measure proof-DAG size and depth.

Its output has no authority. A simplified `RawCertificate` must be replayed by
`nmlt-kernel::check`; recomputing a claimed digest is only serialization
bookkeeping. This crate must never become a dependency of `nmlt-kernel`.
