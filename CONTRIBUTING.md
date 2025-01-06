# Contributing Guidelines

Thank you for your interest in contributing to our project. Whether it's a bug report, new feature, correction, or additional
documentation, we greatly value feedback and contributions from our community.

Please read through this document before submitting any issues or pull requests to ensure we have all the necessary
information to effectively respond to your bug report or contribution.


## Reporting Bugs/Feature Requests

We welcome you to use the GitHub issue tracker to report bugs or suggest features.

When filing an issue, please check existing open, or recently closed, issues to make sure somebody else hasn't already
reported the issue. Please try to include as much information as you can. Details like these are incredibly useful:

* A reproducible test case or series of steps
* The version of our code being used
* Any modifications you've made relevant to the bug
* Anything unusual about your environment or deployment


## Contributing via Pull Requests
Contributions via pull requests are much appreciated. Before sending us a pull request, please ensure that:

1. You are working against the latest source on the *main* branch.
2. You check existing open, and recently merged, pull requests to make sure someone else hasn't addressed the problem already.
3. You open an issue to discuss any significant work - we would hate for your time to be wasted.

To send us a pull request, please:

1. Fork the repository.
2. Modify the source; please focus on the specific change you are contributing. If you also reformat all the code, it will be hard for us to focus on your change.
3. Ensure local tests pass.
4. Commit to your fork using clear commit messages.
5. Send us a pull request, answering any default questions in the pull request interface.
6. Pay attention to any automated CI failures reported in the pull request, and stay involved in the conversation.

GitHub provides additional document on [forking a repository](https://help.github.com/articles/fork-a-repo/) and
[creating a pull request](https://help.github.com/articles/creating-a-pull-request/).

### Prerequisite for a pull request
As required by our security team, it's essential to install [git-secrets](https://github.com/awslabs/git-secrets) in your development environment.
This prevents the inadvertent committing of credentials into the repository. Please follow the instructions [here](https://github.com/awslabs/git-secrets#installing-git-secrets) to install git-secretes.
If you have concerns regarding its requirement, kindly consult the maintainers.


### Recommended tools to create a pull request
We use GitHub Actions to enforce some restrictions when we merge a pull request,
which maintains the quality of our repository code.
Therefore, we recommend using the following tools to accommodate smoothly merge process of your pull request.


#### Local development
For local development, you can use `rustfmt` and `clippy` to maintain the quality of our source code.
Also, it is enforced by our GitHub Actions.
Before starting, please ensure both components are installed;

```shell
rustup component add rustfmt clippy
```

Additionally, we use `pre-commit` hooks to execute automated linting and basic checks.
Please set it up before creating a commit;

```shell
brew install pre-commit # (or appropriate for your platform: https://pre-commit.com/)
pre-commit install
```

We also use [trycmd](https://crates.io/crates/trycmd) to conduct snapshot testing for CLI.
If the snapshot is needed to be updated, run command;

MacOS and Linux
```shell
TRYCMD=overwrite cargo test --test cli_tests
```

Windows (PowerShell)
```powershell
$Env:TRYCMD='overwrite'
cargo test --test cli_tests
[Environment]::SetEnvironmentVariable('TRYCMD',$null)
```

Please note that we use different snapshots for the Windows environment.
Therefore, you need to update snapshot both Linux and Windows after modifying the behavior of CLI output.

#### Bot command for a pull request
*We temporarily disable the bot command currently.*

If you want to update snapshots of commands, you can use the bot command `/snapshot` in your pull request.
Please note that you must type a command exactly as written.

The bot creates diff files for both Windows and Linux. You can use generated diff to patch your commit.

For example, if you have developed in a Linux environment and modified the command option,
you must also update the snapshot for the Windows environment.
In this case, you can create a pull request for draft mode and execute `/snapshot` to create a diff file for Windows.
Generated diff can be copied into a file and applied by `git diff <file-name>` command.

#### Fuzz testing
We use fuzz testing to verify the implementation of our parser.

You can find resources related to fuzz testing in Rust in the following links:

* [Rust Fuzz Book](https://rust-fuzz.github.io/book/introduction.html)
* [Instrumentation-based Code Coverage - The rustc book](https://doc.rust-lang.org/stable/rustc/instrument-coverage.html)

If you want to quickly run the fuzz tests with libfuzzer, you can use the following commands:

```bash
# Install cargo-fuzz
cargo +nightly install cargo-fuzz
# Start a series of fuzz tests
cargo +nightly fuzz run -j $(nproc) parse_dynein_format -- -dict=fuzz/fuzz_dict
cargo +nightly fuzz run -j $(nproc) parse_remove_action -- -dict=fuzz/fuzz_dict
cargo +nightly fuzz run -j $(nproc) parse_set_action -- -dict=fuzz/fuzz_dict
cargo +nightly fuzz run -j $(nproc) parse_sort_key_with_fallback -- -dict=fuzz/fuzz_dict
cargo +nightly fuzz run -j $(nproc) parse_sort_key_with_suggest -- -dict=fuzz/fuzz_dict
```

## Finding contributions to work on
Looking at the existing issues is a great way to find something to contribute on. As our projects, by default, use the default GitHub issue labels (enhancement/bug/duplicate/help wanted/invalid/question/wontfix), looking at any 'help wanted' issues is a great place to start.


## Code of Conduct
This project has adopted the [Amazon Open Source Code of Conduct](https://aws.github.io/code-of-conduct).
For more information see the [Code of Conduct FAQ](https://aws.github.io/code-of-conduct-faq) or contact
opensource-codeofconduct@amazon.com with any additional questions or comments.


## Security issue notifications
If you discover a potential security issue in this project we ask that you notify AWS/Amazon Security via our [vulnerability reporting page](http://aws.amazon.com/security/vulnerability-reporting/). Please do **not** create a public github issue.


## Licensing

See the [LICENSE](LICENSE) file for our project's licensing. We will ask you to confirm the licensing of your contribution.
