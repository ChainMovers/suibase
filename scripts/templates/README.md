!!!!Warning!!!!
If you are looking to configure sui-base, please check instead to modify the sui-base.yaml into the workdir.

The templates directories are intended for the developers of sui-base itself.

What are the templates used for?
================================
All files from a sub-directory are copied *once* when a workdir is *created* with the same name.

sui-exec and workdir-exec are shims at two key "abstraction" point (already useful at making the workdir more generic). They may also allow the user to hook their own scripts (future feature?).

The shims are also copied (and updated if older version) into every workdir.