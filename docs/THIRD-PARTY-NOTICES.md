# Third-party notices

All asset paths below are relative to the repository root.

The executable includes the original project's pinned Windows tool binaries. These components remain licensed by their respective authors. Full upstream notices are included in `assets/licenses` and embedded in the executable; run `proto-builder licenses` to display them.

| Component | Version | Upstream source | Notice |
| --- | --- | --- | --- |
| Protocol Buffers / protoc | 3.15.7 | https://github.com/protocolbuffers/protobuf/tree/v3.15.7 | assets/licenses/protobuf.txt |
| protoc-gen-go | 1.27.1 | https://github.com/protocolbuffers/protobuf-go/tree/v1.27.1 | assets/licenses/protobuf-go.txt |
| protoc-gen-go-grpc | 1.2.0 | https://github.com/grpc/grpc-go/tree/cmd/protoc-gen-go-grpc/v1.2.0 | assets/licenses/grpc-go.txt |
| protoc-gen-doc | 1.5.0 | https://github.com/pseudomuto/protoc-gen-doc/tree/v1.5.0 | assets/licenses/protoc-gen-doc.txt |

Rust dependencies are locked by Cargo.lock. Their package names, versions, declared SPDX licenses, source URLs and packaged root license texts are collected in `assets/licenses/rust-dependencies.txt`. This file also covers dependency packages for future platform targets. Regenerate these notices when dependencies change. The main program's source license has not been selected by the owner; this notice does not assign one.

The bundled executables are identified by the SHA256 values in `assets/windows-x86_64/manifest.json`. They are reused without modification to preserve existing generated file bytes. Source dependencies and upstream notices do not replace this identity check.
