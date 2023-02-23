# Web package

In order to avoid `src/generated/google/protobuf/descriptor.ts(1134,14): error TS7056: The inferred type of this node exceeds the maximum length the compiler will serialize. An explicit type annotation is needed.` error, we need to specify entry point to `tsup`. This allow use to build only module we want.
