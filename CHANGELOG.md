# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## Fixed
- Make counter value signed when tracking expressions to prevent underflow

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
