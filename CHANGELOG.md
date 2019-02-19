# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Added
- `#[builder(default_code = "...")]` syntax for defaults that cannot be parsed
  as attributes no matter what.

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
