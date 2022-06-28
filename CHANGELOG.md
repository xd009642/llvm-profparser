# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Changed
- Made instrumentation profile parsing failure message more serious
- Made hit adding in report use `saturating_add` to prevent overflow

### Fixed
- Make counter value signed when tracking expressions to prevent underflow
- Multiply max counters by counter size when comparing to counter delta

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
