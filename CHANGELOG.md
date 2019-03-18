# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Added
- `#![no_std]` is now supported out of the box. (You don’t need to opt into any features, it just works.)
- [**BREAKING**] a `default_code` expression can now refer to the values of earlier fields by name.
  (This is extremely unlikely to break your code, but could in theory due to shadowing.)
- `#[builder(exclude)]` on fields, to not provide a method to set that field.
- Control of documentation:
  - `#[builder(doc = "…")]` on fields, to document the field’s method on the builder. Unlike `#[doc]`, you can currently only have one value rather than one attribute per line; but that’s not a big deal since you don’t get to use the `///` sugar anyway. Just use a multiline string.
  - `#[builder(doc, builder_method_doc = "…", builder_type_doc = "…", build_method_doc = "…")]` on structs:
    - `doc` unhides the builder type from the documentation.
	- `builder_method_doc = "…"` replaces the default documentation that will be generated for the builder() method of the type for which the builder is being generated.
	- `builder_type_doc = "…"` replaces the default documentation that will be generated for the builder type. Implies `doc`.
	- `build_method_doc = "…"` replaces the default documentation that will be generated for the build() method of the builder type. Implies `doc`.

### Changed
- [**BREAKING**] Renamed the generated builder type from `TypedBuilder_BuilderFor_Foo` to `FooBuilder`, for improved ergonomics, especially when you enable documentation of the builder type. You can also now change it to something else with `#[builder(name = SomethingElse)]` on the type you are deriving TypedBuilder on.
  - Generic identifiers were also changed, from `TypedBuilder_genericType_x` to `__x`. This is still expected to avoid all name collisions, but is easier to read in the builder type docs if you enable them.
  - Renamed the conversion helper trait for documentation purposes (`TypedBuilder_conversionHelperTrait_Foo` to `FooBuilder_Optional`), and its method name for simpler code.

## 0.3.0 - 2019-02-19
### Added
- `#[builder(default_code = "...")]` syntax for defaults that cannot be parsed
  as attributes no matter what.

### Changed
- Move the docs from the crate to the custom derive proc macro.

## 0.2.0 - 2019-02-06
### Changed
- Upgraded `syn` version to support Rust 2018.
- [**BREAKING**] Changed attribute style to `#[builder(...)]`:
  - `#[default]` -> `#[builder(default)]`
  - `#[default=...]` -> `#[builder(default=...)]`
- [**BREAKING**] `default` no longer needs to be a string.
  - But you need to change your code anyways because the attribute style was changed.

## 0.1.1 - 2018-07-24
### Fixed
- Allow missing docs in structs that derive `TypedBuilder`.

## 0.1.0 - 2017-10-05
### Added
- Custom derive for generating the builder pattern.
- All setters are accepting `Into` values.
- Compile time verification that all fields are set before calling `.build()`.
- Compile time verification that no field is set more than once.
- Ability to annotate fields with `#[default]` to make them optional and specify a default value when the user does not set them.
- Generates simple documentation for the `.builder()` method.
