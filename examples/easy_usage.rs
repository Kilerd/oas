use oas::{builders, Referenceable, PathItem, Tag, Server};

fn main() {
    // Example 1: Simple API using builder pattern
    let api = builders::api("My Pet Store API", "1.0.0")
        .with_description("A sample Pet Store API")
        .add_server(Server::new("https://api.petstore.com")
            .with_description("Production server"))
        .add_tag(Tag::with_description("pets", "Pet operations"))
        .add_path("/pets", PathItem::new()
            .with_get(builders::get("List all pets")
                .tag("pets")
                .operation_id("listPets")
                .parameter(Referenceable::query_param("limit")
                    .with_schema(Referenceable::integer_schema())
                    .with_description("How many items to return at one time"))
                .response("200", Referenceable::ok("A list of pets")
                    .with_content({
                        let mut content = std::collections::BTreeMap::new();
                        content.insert("application/json".to_string(),
                            oas::MediaType::new()
                                .with_schema(Referenceable::array_schema()));
                        content
                    }))
                .build())
            .with_post(builders::post("Create a pet")
                .tag("pets")
                .operation_id("createPet")
                .request_body(Referenceable::json_body(
                    Referenceable::schema_ref("Pet")))
                .build()))
        .add_path("/pets/{petId}", PathItem::new()
            .with_get(builders::get("Info for a specific pet")
                .tag("pets")
                .operation_id("showPetById")
                .parameter(Referenceable::path_param("petId")
                    .with_schema(Referenceable::string_schema())
                    .with_description("The id of the pet to retrieve"))
                .build()));

    println!("Generated OpenAPI spec:");
    println!("{}", api.to_string());

    // Example 2: Using convenience methods
    let simple_api = oas::OpenAPIV3::new(
        oas::Info::new("Simple API", "0.1.0")
            .with_description("A very simple API example")
    )
    .add_server(Server::new("http://localhost:8080"))
    .add_path("/health", PathItem::new()
        .with_get(oas::Operation::new(
            oas::Responses::new()
                .with_status("200", Referenceable::ok("Health check"))
        ).with_summary("Health check endpoint")));

    println!("\nSimple API:");
    println!("{}", simple_api.to_string());
}