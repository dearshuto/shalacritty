fn main() {
    tonic_build::compile_protos("proto/profile.proto").unwrap();

    tonic_build::configure()
        .build_server(false)
        .build_client(false)
        .type_attribute(
            "profile_core.Profile",
            "#[derive(serde::Deserialize, serde::Serialize)]",
        )
        .type_attribute(
            "profile_core.ProfileListRequest",
            "#[derive(serde::Deserialize, serde::Serialize)]",
        )
        .type_attribute(
            "profile_core.ProfileListResponse",
            "#[derive(serde::Deserialize, serde::Serialize)]",
        )
        .compile_protos(&["proto/profile.proto"], &Vec::<std::path::PathBuf>::new())
        .unwrap();
}
