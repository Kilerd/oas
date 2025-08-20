# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2025-08-20

### Added
- **Comprehensive convenience constructors** for all major types (`new()` methods with fluent APIs)
- **Builder pattern support** with `OperationBuilder` for complex operation construction
- **Quick builders module** (`builders::api`, `builders::get`, `builders::post`, etc.)
- **Referenceable helper methods** for common patterns:
  - Type-specific constructors (`query_param`, `path_param`, `schema_ref`)
  - Chainable methods (`with_schema`, `with_description`, `with_content`)
- **Schema shortcuts** (`Schema::string()`, `Schema::integer()`, etc.)
- **Default implementations** for appropriate types
- **Extensive documentation** with examples for all public APIs
- **Comprehensive README.md** with usage examples and API reference
- **Usage guide** (`USAGE_GUIDE.md`) with detailed tutorials
- **Example files** demonstrating various usage patterns

### Changed
- **Improved ergonomics** - significantly reduced boilerplate code required
- **Enhanced type safety** - better inference and cleaner APIs
- **Documentation coverage** - every public item now has comprehensive docs

### Technical Details
- All convenience methods maintain 100% backward compatibility
- Builder patterns use move semantics for efficient chaining
- Examples compile and run successfully
- All documentation tests pass

This release represents a major usability improvement while maintaining full API compatibility.

## [0.1.1] - Previous Release
- Basic OpenAPI 3.0 support
- Core serialization/deserialization functionality

## [0.1.0] - Initial Release  
- Initial implementation of OpenAPI 3.0 specification in Rust
- Basic struct definitions and serde support