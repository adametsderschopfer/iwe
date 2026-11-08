use super::*;
use std::ffi::OsStr;

#[test]
fn priorities_match_each_supported_platform() {
    assert_eq!(
        priority("macos"),
        Some([Backend::Metal, Backend::Vulkan, Backend::OpenGl].as_slice())
    );
    assert_eq!(
        priority("windows"),
        Some([Backend::Vulkan, Backend::DirectX12, Backend::OpenGl].as_slice())
    );
    assert_eq!(
        priority("linux"),
        Some([Backend::Vulkan, Backend::OpenGl].as_slice())
    );
    assert!(priority("unsupported").is_none());
}

#[test]
fn failed_initialization_tries_the_next_backend_on_every_platform() {
    for os in ["macos", "windows", "linux"] {
        let backends = priority(os).unwrap();

        for available in 0..backends.len() {
            let mut tried = Vec::new();
            let result = try_backends(backends, |backend| {
                tried.push(backend);
                Ok(if backend == backends[available] {
                    0
                } else {
                    GRAPHICS_UNAVAILABLE
                })
            });

            assert_eq!(result.unwrap(), Some(0));
            assert_eq!(tried, backends[..=available]);
        }
    }
}

#[test]
fn unrelated_application_failures_do_not_retry() {
    let mut tried = Vec::new();
    let result = try_backends(priority("windows").unwrap(), |backend| {
        tried.push(backend);
        Ok(1)
    });

    assert_eq!(result.unwrap(), Some(1));
    assert_eq!(tried, [Backend::Vulkan]);
}

#[test]
fn all_unavailable_backends_are_attempted_once() {
    let backends = priority("macos").unwrap();
    let mut tried = Vec::new();
    let result = try_backends(backends, |backend| {
        tried.push(backend);
        Ok(GRAPHICS_UNAVAILABLE)
    });

    assert_eq!(result.unwrap(), None);
    assert_eq!(tried, backends);
}

#[test]
fn process_start_failure_does_not_retry() {
    let mut attempts = 0;
    let result = try_backends(priority("linux").unwrap(), |_| {
        attempts += 1;
        Err(io::Error::new(io::ErrorKind::PermissionDenied, "test"))
    });

    assert_eq!(result.unwrap_err().kind(), io::ErrorKind::PermissionDenied);
    assert_eq!(attempts, 1);
}

#[test]
fn child_gets_exact_backend_and_original_arguments() {
    for (backend, expected) in [
        (Backend::Metal, "metal"),
        (Backend::Vulkan, "vulkan"),
        (Backend::DirectX12, "dx12"),
        (Backend::OpenGl, "gl"),
    ] {
        let args = [OsString::from("a path with spaces")];
        let command = child_command(Path::new("iwe"), &args, backend);
        let variables: std::collections::BTreeMap<_, _> = command.get_envs().collect();

        assert_eq!(command.get_args().collect::<Vec<_>>(), args);
        assert_eq!(variables[OsStr::new(CHILD_MARKER)], Some(OsStr::new("1")));
        assert_eq!(
            variables[OsStr::new("ICED_BACKEND")],
            Some(OsStr::new("wgpu"))
        );
        assert_eq!(
            variables[OsStr::new("WGPU_BACKEND")],
            Some(OsStr::new(expected))
        );
    }
}

#[test]
fn only_graphics_creation_errors_trigger_fallback() {
    let i18n = I18n::new(crate::i18n::Language::English);
    let graphics_error = iced_test::renderer::graphics::Error::List(Vec::new());

    assert_eq!(
        application_exit_code(
            &i18n,
            Err(iced::Error::GraphicsCreationFailed(graphics_error))
        ),
        GRAPHICS_UNAVAILABLE
    );
    assert_eq!(application_exit_code(&i18n, Ok(())), 0);
    assert_eq!(
        application_exit_code(
            &i18n,
            Err(iced::Error::ExecutorCreationFailed(io::Error::other(
                "test"
            )))
        ),
        1
    );
}
