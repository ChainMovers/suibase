# !!!!Warning!!!!
If you are looking to configure suibase, please check instead to modify the suibase.yaml into the workdir.

The templates are intended for the developers of suibase itself.

# What are the templates for?
All files from a sub-directory are copied *once* when a workdir is *created* with the same name.

sui-exec and workdir-exec are shims of two "abstractions" (already useful at making the workdir more generic). They may also allow the user to hook their own scripts (future feature?).

The shims are also copied (and updated as needed) into every workdir.