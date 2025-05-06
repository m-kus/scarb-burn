use anyhow::{bail, Context};
use cairo_lang_runner::profiling::{
    ProcessedProfilingInfo, ProfilingInfoProcessor, ProfilingInfoProcessorParams,
};
use cairo_lang_runner::short_string::as_cairo_short_string;
use cairo_lang_runner::{
    Arg, ProfilingInfoCollectionConfig, RunResultValue, SierraCasmRunner, StarknetState,
};
use cairo_lang_sierra::program::VersionedProgram;
use cairo_lang_utils::ordered_hash_map::OrderedHashMap;

/// Load Sierra program from source, run it and generate a profile.
pub fn profile(
    program: VersionedProgram,
    program_args: Vec<Arg>,
) -> anyhow::Result<ProcessedProfilingInfo> {
    let sierra_program = program
        .into_v1()
        .with_context(|| "failed to convert to v1")?;
    let gas_enabled = sierra_program.program.requires_gas_counter();

    let runner = SierraCasmRunner::new(
        sierra_program.program.clone(),
        if gas_enabled {
            Some(Default::default())
        } else {
            None
        },
        Default::default(),
        Some(ProfilingInfoCollectionConfig {
            collect_scoped_sierra_statement_weights: true,
            ..Default::default()
        }),
    )?;

    let entrypoint = runner.find_function("main").with_context(|| {
        format!(
            r#"
            Make sure you have the following in Scarb.toml:

            [cairo]
            sierra-replace-ids = true

            Error"#
        )
    })?;

    let result = runner
        .run_function_with_starknet_context(
            entrypoint,
            vec![Arg::Array(program_args), Arg::Array(vec![])],
            if gas_enabled { Some(usize::MAX) } else { None },
            StarknetState::default(),
        )
        .with_context(|| "failed to run the function")?;

    if let RunResultValue::Panic(values) = result.value {
        let msg = values
            .iter()
            .map(|v| as_cairo_short_string(v).unwrap_or_else(|| v.to_string()))
            .collect::<Vec<_>>()
            .join(", ");
        bail!("panicked with [{msg}]")
    }

    let profiling_processor = ProfilingInfoProcessor::new(
        None,
        sierra_program.program,
        Default::default(),
        ProfilingInfoProcessorParams {
            min_weight: 1,
            process_by_statement: false,
            process_by_concrete_libfunc: false,
            process_by_generic_libfunc: false,
            process_by_user_function: false,
            process_by_original_user_function: false,
            process_by_cairo_function: false,
            process_by_stack_trace: false,
            process_by_cairo_stack_trace: false,
            process_by_scoped_statement: true,
        },
    );
    let mut processed_profiling_info =
        profiling_processor.process(result.profiling_info.as_ref().unwrap());

    // Adjust weights according to the builtins/libfuncs table
    if let Some(scoped_sierra_statement_weights) = processed_profiling_info
        .scoped_sierra_statement_weights
        .as_mut()
    {
        adjust_weights(scoped_sierra_statement_weights);
    }

    Ok(processed_profiling_info)
}

fn adjust_weights(weights: &mut OrderedHashMap<Vec<String>, usize>) {
    weights.iter_mut().for_each(|(k, v)| {
        //println!("{}: {}", k.join(" -> "), v);
    });
}

#[cfg(test)]
mod tests {
    use cairo_lang_utils::bigint::BigUintAsHex;

    use super::*;

    #[test]
    fn test_adjust_weights() {
        let source = include_str!("../tests/data/falcon.sierra.json");
        let args_source = include_str!("../tests/data/falcon_args.json");
        let program = serde_json::from_str::<VersionedProgram>(source)
            .expect("failed to deserialize Sierra program");
        let arguments = serde_json::from_str::<Vec<BigUintAsHex>>(args_source)
            .expect("failed to deserialize arguments");
        let args: Vec<Arg> = arguments
            .into_iter()
            .map(|arg| Arg::Value(arg.value.into()))
            .collect();
        let _ = profile(program, args).expect("failed to profile");
    }
}
