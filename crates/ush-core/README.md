# Shared USH core

This is the Rust core from `Animuni-Express/USH`, copied from the existing
checkout at Unknown's integration point. Unknown's CLI and browser adapter both
depend on this tracked crate. The original `USH/` checkout is preserved and is
not used by these builds. Browser file operations use `vfs`; native operations
retain the standard library backend. Shell command dispatch is shared.
