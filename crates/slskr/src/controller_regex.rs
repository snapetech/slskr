use crate::config::ControllerProfile;
use std::time::Duration;

#[derive(Clone, Debug)]
pub(crate) struct ControllerRegex {
    pub(crate) expression: String,
    matcher: crate::dotnet_regex::DotNetRegex,
    match_timeout: Option<Duration>,
}

impl ControllerRegex {
    pub(crate) fn compile_with_timeout(
        expression: &str,
        case_sensitive: bool,
        match_timeout: Option<Duration>,
    ) -> Result<Self, String> {
        let matcher = crate::dotnet_regex::DotNetRegex::compile(expression, case_sensitive)
            .map_err(|error| format!("invalid regular expression {expression:?}: {error}"))?;
        Ok(Self {
            expression: expression.to_owned(),
            matcher,
            match_timeout,
        })
    }

    pub(crate) fn is_match(&self, value: &str) -> bool {
        match self.match_timeout {
            Some(timeout) => self
                .matcher
                .is_match_with_timeout(value, timeout)
                .unwrap_or(false),
            None => self.matcher.is_match(value).unwrap_or(false),
        }
    }
}

pub(crate) const NATIVE_REGEX_MATCH_TIMEOUT: Duration = Duration::from_millis(250);

fn controller_regex_timeout(target: ControllerProfile) -> Option<Duration> {
    (target == ControllerProfile::Native).then_some(NATIVE_REGEX_MATCH_TIMEOUT)
}

pub(crate) fn compile_controller_regexes(
    expressions: &[String],
    case_sensitive: bool,
    target: ControllerProfile,
) -> Result<Vec<ControllerRegex>, String> {
    let match_timeout = controller_regex_timeout(target);
    expressions
        .iter()
        .map(|expression| {
            ControllerRegex::compile_with_timeout(expression, case_sensitive, match_timeout)
        })
        .collect()
}

pub(crate) fn compile_controller_regexes_for_request(
    expressions: Vec<String>,
    case_sensitive: bool,
    target: ControllerProfile,
) -> Result<Vec<ControllerRegex>, String> {
    std::thread::Builder::new()
        .name("slskr-regex-compile".to_owned())
        .stack_size(16 * 1024 * 1024)
        .spawn(move || compile_controller_regexes(&expressions, case_sensitive, target))
        .map_err(|error| format!("failed to start regular-expression compilation: {error}"))?
        .join()
        .map_err(|_| "regular-expression compilation panicked".to_owned())?
}
