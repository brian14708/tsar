fn main() {
    protobuf_codegen::Codegen::new()
        .pure()
        .include("src")
        .inputs(["src/tsar.proto"])
        .cargo_out_dir("pb")
        .run_from_script();
}
