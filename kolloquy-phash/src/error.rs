/// Log an error, using tracing::error if the "tracing" feature is enabled or by
/// printing to stderr if it is not.
#[macro_export]
macro_rules! raise_error {
    (
        name : $name:literal,
        target : $target:literal,
        $( $fmt:tt )+ // Just consume everything after lol
    ) => {
        {
            #[cfg(feature = "tracing")]
            tracing::error!(
                name: $name,
                target: $target,
                $( $fmt )+
            );

            #[cfg(not(feature = "tracing"))]
            {
                eprint!("Error {} in {}: ", $name, $target);
                eprintln!($( $fmt )+);
            };
        }
    };

    (
        name : $name:literal,
        $( $fmt:tt )+ // Just consume everything after lol
    ) => {
        {
            #[cfg(feature = "tracing")]
            tracing::error!(
                name: $name,
                $( $fmt )+
            );

            #[cfg(not(feature = "tracing"))]
            {
                eprint!("Error {}: ", $name);
                eprintln!($( $fmt )+);
            };
        };
    };
}

/// Log a warning, using tracing::warn if the "tracing" feature is enabled or by
/// printing to stderr if it is not.
#[macro_export]
macro_rules! raise_warning {
    (
        name : $name:literal,
        target : $target:literal,
        $( $fmt:tt )+ // Just consume everything after lol
    ) => {
        {
            #[cfg(feature = "tracing")]
            tracing::warn!(
                name: $name,
                target: $target,
                $( $fmt )+
            );

            #[cfg(not(feature = "tracing"))]
            {
                eprint!("Warning {} in {}: ", $name, $target);
                eprintln!($( $fmt )+);
            };
        }
    };

    (
        name : $name:literal,
        $( $fmt:tt )+ // Just consume everything after lol
    ) => {
        {
            #[cfg(feature = "tracing")]
            tracing::warn!(
                name: $name,
                $( $fmt )+
            );

            #[cfg(not(feature = "tracing"))]
            {
                eprint!("Warning {}: ", $name);
                eprintln!($( $fmt )+);
            };
        };
    };
}

/// Log info, using tracing::info if the "tracing" feature is enabled or by
/// printing to stderr if it is not.
#[macro_export]
macro_rules! log_info {
    (
        name : $name:literal,
        target : $target:literal,
        $( $fmt:tt )+ // Just consume everything after lol
    ) => {
        {
            #[cfg(feature = "tracing")]
            tracing::info!(
                name: $name,
                target: $target,
                $( $fmt )+
            );

            #[cfg(not(feature = "tracing"))]
            {
                eprint!("INFO {} in {}: ", $name, $target);
                eprintln!($( $fmt )+);
            };
        }
    };

    (
        name : $name:literal,
        $( $fmt:tt )+ // Just consume everything after lol
    ) => {
        {
            #[cfg(feature = "tracing")]
            tracing::info!(
                name: $name,
                $( $fmt )+
            );

            #[cfg(not(feature = "tracing"))]
            {
                eprint!("INFO {}: ", $name);
                eprintln!($( $fmt )+);
            };
        };
    };
}

/// Log a trace, using tracing::trace if the "tracing" feature is enabled or by
/// printing to stderr if it is not.
#[macro_export]
macro_rules! log_trace {
    (
        name : $name:literal,
        target : $target:literal,
        $( $fmt:tt )+ // Just consume everything after lol
    ) => {
        {
            #[cfg(feature = "tracing")]
            tracing::trace!(
                name: $name,
                target: $target,
                $( $fmt )+
            );

            #[cfg(not(feature = "tracing"))]
            {
                eprint!("Trace {} in {}: ", $name, $target);
                eprintln!($( $fmt )+);
            };
        }
    };

    (
        name : $name:literal,
        $( $fmt:tt )+ // Just consume everything after lol
    ) => {
        {
            #[cfg(feature = "tracing")]
            tracing::trace!(
                name: $name,
                $( $fmt )+
            );

            #[cfg(not(feature = "tracing"))]
            {
                eprint!("Trace {}: ", $name);
                eprintln!($( $fmt )+);
            };
        };
    };
}
