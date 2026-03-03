pub struct GeneratorDef {
    pub name: &'static str,
    pub compile_prefix: &'static str,
}

pub const SERVER_GENERATORS: &[GeneratorDef] = &[
    GeneratorDef {
        name: "aspnetcore",
        compile_prefix: "build-",
    },
    GeneratorDef {
        name: "go-server",
        compile_prefix: "build-",
    },
    GeneratorDef {
        name: "kotlin-spring",
        compile_prefix: "build-",
    },
    GeneratorDef {
        name: "python-fastapi",
        compile_prefix: "build-",
    },
    GeneratorDef {
        name: "spring",
        compile_prefix: "build-",
    },
    GeneratorDef {
        name: "typescript-nestjs",
        compile_prefix: "build-",
    },
];

pub const CLIENT_GENERATORS: &[GeneratorDef] = &[
    GeneratorDef {
        name: "csharp",
        compile_prefix: "build-client-",
    },
    GeneratorDef {
        name: "go",
        compile_prefix: "build-client-",
    },
    GeneratorDef {
        name: "java",
        compile_prefix: "build-client-",
    },
    GeneratorDef {
        name: "kotlin",
        compile_prefix: "build-client-",
    },
    GeneratorDef {
        name: "python",
        compile_prefix: "build-client-",
    },
    GeneratorDef {
        name: "typescript-axios",
        compile_prefix: "build-client-",
    },
    GeneratorDef {
        name: "typescript-fetch",
        compile_prefix: "build-client-",
    },
    GeneratorDef {
        name: "typescript-node",
        compile_prefix: "build-client-",
    },
];

pub fn server_names() -> Vec<&'static str> {
    SERVER_GENERATORS.iter().map(|g| g.name).collect()
}

pub fn client_names() -> Vec<&'static str> {
    CLIENT_GENERATORS.iter().map(|g| g.name).collect()
}

pub fn all_server_names(custom: &[crate::custom::CustomGeneratorDef]) -> Vec<String> {
    let mut names: Vec<String> = SERVER_GENERATORS
        .iter()
        .map(|g| g.name.to_string())
        .collect();
    names.extend(crate::custom::server_names(custom));
    names
}

pub fn all_client_names(custom: &[crate::custom::CustomGeneratorDef]) -> Vec<String> {
    let mut names: Vec<String> = CLIENT_GENERATORS
        .iter()
        .map(|g| g.name.to_string())
        .collect();
    names.extend(crate::custom::client_names(custom));
    names
}
