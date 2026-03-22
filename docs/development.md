# Logs

You can find logs at one of these locations

```
# linux
/home/alice/.local/share/heta/heta.log
/home/alice/heta/heta.log

# macos
/Users/alice/Library/Application Support/heta/heta.log
```

If you want to see debug logs, configure the `RUST_LOG` environment variable with `heta=debug,aws_config=warn,aws_smithy_runtime=warn` to reduce noise.
