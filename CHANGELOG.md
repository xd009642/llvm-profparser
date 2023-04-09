# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Changed
- Removed another two redundant hash computations

## [0.3.3] - 2023-04-04
### Changed
- Cache name hash calculation to avoid recomputing (perf)

## [0.3.2] - 2023-03-29
### Added
- llvm-16 test files to ensure support doesn't break

### Fixed
- Fixed parsing of branch region counters

## [0.3.1] - 2023-01-23
### Added
- Debug logging via tracing

### Fixed
- Build on 32 bit architectures

## [0.3.0] - 2022-09-26
### Added
- `InstrumenationProfile::is_empty` to detect when there are no records
- Fuzzing module for profile files

### Changed
- Added anyhow and use in place of `Result<T, Box<dyn Error>>`
- Make error type for profiles `VerboseError`

### Fixed
- Handle merging of completely disjoint records - now profiles generated from multiple
applications are accurately merged
- Handle invalid Hash enum variant in `IndexedProfile`

## [0.2.0] - 2022-09-11
### Changed
- Made instrumentation profile parsing failure message more serious
- Made hit adding in report use `saturating_add` to prevent overflow

### Fixed
- Make counter value signed when tracking expressions to prevent underflow
- Multiply max counters by counter size when comparing to counter delta
- Fixed handling of profile instrumentation not tied to a counter with source location
- Incorrect matching on hashes for instrumentation profile merging

## [0.1.1] - 2022-06-26
### Added
- Detection of memory profiling

### Fixed
- Counter offsetting in raw profiles now implemented
- Counter size for byte coverage now correct
- Text profile now handles carriage returns

## [0.1.0] - 2022-06-05
### Added
- Parsing of indexed, text and raw profiles (llvm version 11, 12, 13, 14)
- Parsing of instrumented binary and generating line coverage reports
