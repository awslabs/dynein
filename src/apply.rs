/*
 * Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
 *
 * Licensed under the Apache License, Version 2.0 (the "License").
 * You may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use log::error;
use std::path::Path;

pub trait PathExistenceChecker {
    fn path_exists(&self, path: &str) -> bool;
}

pub struct RealPathExistenceChecker;

impl PathExistenceChecker for RealPathExistenceChecker {
    fn path_exists(&self, path: &str) -> bool {
        Path::new(path).exists()
    }
}

fn check_template(checker: &dyn PathExistenceChecker) -> Option<&'static str> {
    let acceptable_files = ["cfn.yaml", "cfn.yml", "cfn.json"];
    acceptable_files
        .iter()
        .find(|&&file_name| checker.path_exists(file_name))
        .copied()
}

pub fn apply(checker: &dyn PathExistenceChecker) {
    match check_template(checker) {
        Some(file_name) => todo!("{file_name}"),
        None => {
            error!("cfn.yaml or cfn.yml or cfn.json is not exist");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub struct MockPathExistenceChecker {
        mock_file: String,
    }

    impl MockPathExistenceChecker {
        pub fn new(mock_file: &str) -> Self {
            Self {
                mock_file: mock_file.to_string(),
            }
        }
    }

    impl PathExistenceChecker for MockPathExistenceChecker {
        fn path_exists(&self, path: &str) -> bool {
            path == self.mock_file
        }
    }

    #[test]
    fn test_check_template_with_acceptable_file() {
        let mock_checker = MockPathExistenceChecker::new("cfn.yaml");
        let result = check_template(&mock_checker);
        assert_eq!(result, Some("cfn.yaml"));
    }

    #[test]
    fn test_check_template_without_acceptable_file() {
        let mock_checker = MockPathExistenceChecker::new("some_other_file.txt");
        let result = check_template(&mock_checker);
        assert_eq!(result, None);
    }

    #[test]
    #[should_panic(expected = "not yet implemented: cfn.yaml")]
    fn test_apply_with_acceptable_file() {
        let mock_checker = MockPathExistenceChecker::new("cfn.yaml");
        apply(&mock_checker)
    }
}
