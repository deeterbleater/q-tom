#[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
use qtom_core::RouteCandidate;
use qtom_core::{
    AgentProfile, AgentRouteTable, AgentRuntimeState, RouteError, RouterBackend, RoutingRequest,
    RoutingResult, ScoreCoefficients,
};
use std::ffi::CString;
#[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
use std::ffi::c_void;
use std::marker::PhantomData;
use std::mem::size_of;

pub const CUDA_BACKEND_NAME: &str = "cuda";
pub const ROUTE_AGENTS_K1_KERNEL_NAME: &str = "qtom_route_agents_k1";
pub const ROUTE_AGENTS_K1_PTX: &str = include_str!("../kernels/route_agents.ptx");

const SCAFFOLD_REASON: &str = "CUDA public routing is limited to k=1";
#[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
const CUDA_K_ONE_ONLY_REASON: &str = "CUDA public routing currently supports only k=1";
const CUDA_RUNTIME_AVAILABLE_REASON: &str = "CUDA driver runtime is available";
const CUDA_RUNTIME_FEATURE_DISABLED_REASON: &str = "CUDA runtime feature is disabled";
const CUDA_RUNTIME_UNSUPPORTED_REASON: &str =
    "CUDA runtime detection is unsupported on this platform";
const CUDA_DRIVER_NOT_FOUND_REASON: &str = "CUDA driver library was not found";
const CUDA_DRIVER_SYMBOL_MISSING_REASON: &str = "CUDA driver library is missing a required symbol";
const CUDA_DRIVER_INIT_FAILED_REASON: &str = "CUDA driver initialization failed";
const CUDA_DEVICE_COUNT_FAILED_REASON: &str = "CUDA device count query failed";
const CUDA_NO_DEVICE_REASON: &str = "CUDA driver reported no devices";
const CUDA_DEVICE_GET_FAILED_REASON: &str = "CUDA device query failed";
const CUDA_CONTEXT_RETAIN_FAILED_REASON: &str = "CUDA primary context retain failed";
const CUDA_CONTEXT_SET_CURRENT_FAILED_REASON: &str = "CUDA context activation failed";
const CUDA_DEVICE_ALLOC_FAILED_REASON: &str = "CUDA device allocation failed";
const CUDA_DEVICE_FREE_FAILED_REASON: &str = "CUDA device free failed";
const CUDA_HOST_TO_DEVICE_COPY_FAILED_REASON: &str = "CUDA host-to-device copy failed";
const CUDA_DEVICE_TO_HOST_COPY_FAILED_REASON: &str = "CUDA device-to-host copy failed";
const CUDA_STREAM_CREATE_FAILED_REASON: &str = "CUDA stream create failed";
const CUDA_STREAM_DESTROY_FAILED_REASON: &str = "CUDA stream destroy failed";
const CUDA_STREAM_SYNCHRONIZE_FAILED_REASON: &str = "CUDA stream synchronize failed";
const CUDA_EVENT_CREATE_FAILED_REASON: &str = "CUDA event create failed";
const CUDA_EVENT_DESTROY_FAILED_REASON: &str = "CUDA event destroy failed";
const CUDA_EVENT_RECORD_FAILED_REASON: &str = "CUDA event record failed";
const CUDA_EVENT_SYNCHRONIZE_FAILED_REASON: &str = "CUDA event synchronize failed";
const CUDA_EVENT_ELAPSED_TIME_FAILED_REASON: &str = "CUDA event elapsed time query failed";
const CUDA_MODULE_IMAGE_CONTAINS_NUL_REASON: &str = "CUDA module image contains an interior nul";
const CUDA_MODULE_LOAD_FAILED_REASON: &str = "CUDA module load failed";
const CUDA_MODULE_UNLOAD_FAILED_REASON: &str = "CUDA module unload failed";
const CUDA_FUNCTION_NAME_CONTAINS_NUL_REASON: &str = "CUDA function name contains an interior nul";
const CUDA_FUNCTION_LOOKUP_FAILED_REASON: &str = "CUDA function lookup failed";
const CUDA_INVALID_LAUNCH_CONFIG_REASON: &str = "CUDA kernel launch config is invalid";
const CUDA_KERNEL_LAUNCH_FAILED_REASON: &str = "CUDA kernel launch failed";
const CUDA_BUFFER_LENGTH_MISMATCH_REASON: &str = "CUDA buffer length mismatch";
const CUDA_RESOURCE_SIZE_OVERFLOW_REASON: &str = "CUDA resource size overflow";
const CUDA_PLAN_CONTEXT: &str = "cuda buffer plan";
const CUDA_DEVICE_BUFFER_CONTEXT: &str = "cuda device buffer";
const CUDA_ROUTE_AGENTS_LAUNCH_CONTEXT: &str = "route agents kernel launch";
const ROUTE_AGENTS_K1_BLOCK_DIM_X: u32 = 128;

#[derive(Clone, Debug)]
pub struct CudaRouter {
    name: String,
    route_table: Result<AgentRouteTable, RouteError>,
    coefficients: ScoreCoefficients,
    status: CudaBackendStatus,
}

impl CudaRouter {
    pub fn new(agents: Vec<AgentProfile>, coefficients: ScoreCoefficients) -> Self {
        Self {
            name: CUDA_BACKEND_NAME.to_string(),
            route_table: AgentRouteTable::from_agents(agents),
            coefficients,
            status: CudaBackendStatus::from_runtime(detect_cuda_runtime()),
        }
    }

    pub fn status(&self) -> CudaBackendStatus {
        self.status
    }

    pub fn runtime_status(&self) -> CudaRuntimeStatus {
        self.status.runtime
    }

    pub fn coefficients(&self) -> ScoreCoefficients {
        self.coefficients
    }

    pub fn route_batch_with_timing(
        &self,
        requests: &[RoutingRequest],
        states: &[AgentRuntimeState],
    ) -> Result<CudaTimedRoutingResult, RouteError> {
        let route_table = self.route_table.as_ref().map_err(Clone::clone)?;
        if requests.is_empty() {
            return Ok(CudaTimedRoutingResult {
                results: Vec::new(),
                timing: CudaRouteTimingBreakdown::default(),
            });
        }

        #[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
        {
            let total_start = std::time::Instant::now();
            validate_k_one_inputs(route_table, requests, states)
                .map_err(cuda_route_execution_error_to_route_error)?;

            let runtime_start = std::time::Instant::now();
            let runtime = CudaRuntime::initialize().map_err(cuda_runtime_error_to_route_error)?;
            let runtime_init = runtime_start.elapsed();

            let mut timed = execute_route_agents_k1_timed(
                &runtime,
                route_table,
                self.coefficients,
                requests,
                states,
            )
            .map_err(cuda_route_execution_error_to_route_error)?;
            timed.timing.runtime_init = runtime_init;

            let runtime_teardown_start = std::time::Instant::now();
            drop(runtime);
            timed.timing.runtime_teardown = runtime_teardown_start.elapsed();
            timed.timing.total = total_start.elapsed();
            Ok(timed)
        }
        #[cfg(not(all(feature = "cuda-runtime", any(windows, target_os = "linux"))))]
        {
            let _ = route_table;
            let _ = requests;
            let _ = states;
            Err(RouteError::BackendUnavailable {
                backend: CUDA_BACKEND_NAME,
                reason: runtime_unavailable_error().status().reason,
            })
        }
    }

    pub fn buffer_plan(
        &self,
        request_count: usize,
        k: usize,
    ) -> Result<CudaBufferPlan, RouteError> {
        let route_table = self.route_table.as_ref().map_err(Clone::clone)?;
        CudaBufferPlan::try_new(
            route_table.len(),
            request_count,
            route_table.dimensions(),
            k,
        )
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CudaTimedRoutingResult {
    pub results: Vec<RoutingResult>,
    pub timing: CudaRouteTimingBreakdown,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CudaRouteTimingBreakdown {
    pub total: std::time::Duration,
    pub runtime_init: std::time::Duration,
    pub runtime_teardown: std::time::Duration,
    pub host_prepare: std::time::Duration,
    pub device_allocate: std::time::Duration,
    pub host_to_device: std::time::Duration,
    pub module_stream_setup: std::time::Duration,
    pub kernel_launch_sync: std::time::Duration,
    pub kernel_device: std::time::Duration,
    pub device_to_host: std::time::Duration,
    pub decode: std::time::Duration,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct CudaRouteK1Executor<'runtime> {
    runtime: &'runtime CudaRuntime,
    coefficients: ScoreCoefficients,
    #[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
    module: CudaModule<'runtime>,
    #[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
    workspace: std::cell::RefCell<Option<CudaRouteK1Workspace<'runtime>>>,
}

impl<'runtime> CudaRouteK1Executor<'runtime> {
    pub fn new(
        runtime: &'runtime CudaRuntime,
        coefficients: ScoreCoefficients,
    ) -> Result<Self, RouteError> {
        #[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
        {
            let module = runtime
                .load_route_agents_module()
                .map_err(cuda_runtime_error_to_route_error)?;
            Ok(Self {
                runtime,
                coefficients,
                module,
                workspace: std::cell::RefCell::new(None),
            })
        }
        #[cfg(not(all(feature = "cuda-runtime", any(windows, target_os = "linux"))))]
        {
            let _ = runtime;
            let _ = coefficients;
            Err(RouteError::BackendUnavailable {
                backend: CUDA_BACKEND_NAME,
                reason: runtime_unavailable_error().status().reason,
            })
        }
    }

    pub fn route_batch_with_timing(
        &self,
        route_table: &AgentRouteTable,
        requests: &[RoutingRequest],
        states: &[AgentRuntimeState],
    ) -> Result<CudaTimedRoutingResult, RouteError> {
        if requests.is_empty() {
            return Ok(CudaTimedRoutingResult {
                results: Vec::new(),
                timing: CudaRouteTimingBreakdown::default(),
            });
        }

        #[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
        {
            validate_k_one_inputs(route_table, requests, states)
                .map_err(cuda_route_execution_error_to_route_error)?;
            let plan = CudaBufferPlan::try_new(
                route_table.len(),
                requests.len(),
                route_table.dimensions(),
                1,
            )?;
            let mut workspace = self.workspace.borrow_mut();
            let replace_workspace = workspace
                .as_ref()
                .map(|workspace| workspace.plan != plan)
                .unwrap_or(true);
            if replace_workspace {
                *workspace = Some(
                    CudaRouteK1Workspace::new(self.runtime, plan)
                        .map_err(cuda_runtime_error_to_route_error)?,
                );
            }
            let workspace = workspace
                .as_mut()
                .expect("CUDA route executor workspace should be initialized");

            execute_route_agents_k1_timed_with_workspace(
                self.runtime,
                &self.module,
                workspace,
                route_table,
                self.coefficients,
                requests,
                states,
            )
            .map_err(cuda_route_execution_error_to_route_error)
        }
        #[cfg(not(all(feature = "cuda-runtime", any(windows, target_os = "linux"))))]
        {
            let _ = route_table;
            let _ = requests;
            let _ = states;
            Err(RouteError::BackendUnavailable {
                backend: CUDA_BACKEND_NAME,
                reason: runtime_unavailable_error().status().reason,
            })
        }
    }
}

#[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
#[derive(Debug)]
struct CudaRouteK1Workspace<'runtime> {
    plan: CudaBufferPlan,
    agent_vectors: CudaDeviceBuffer<'runtime, f32>,
    agent_ids: CudaDeviceBuffer<'runtime, u32>,
    request_vectors: CudaDeviceBuffer<'runtime, f32>,
    agent_score_weights: CudaDeviceBuffer<'runtime, f32>,
    availability: CudaDeviceBuffer<'runtime, u32>,
    output_agent_ids: CudaDeviceBuffer<'runtime, u32>,
    output_effective_distances: CudaDeviceBuffer<'runtime, f32>,
    output_base_distances: CudaDeviceBuffer<'runtime, f32>,
    output_flags: CudaDeviceBuffer<'runtime, u32>,
}

#[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
impl<'runtime> CudaRouteK1Workspace<'runtime> {
    fn new(runtime: &'runtime CudaRuntime, plan: CudaBufferPlan) -> Result<Self, CudaRuntimeError> {
        Ok(Self {
            plan,
            agent_vectors: runtime.allocate_device_buffer::<f32>(plan.agent_vector_f32_len)?,
            agent_ids: runtime.allocate_device_buffer::<u32>(plan.agent_id_u32_len)?,
            request_vectors: runtime.allocate_device_buffer::<f32>(plan.request_vector_f32_len)?,
            agent_score_weights: runtime
                .allocate_device_buffer::<f32>(plan.agent_score_weight_f32_len)?,
            availability: runtime.allocate_device_buffer::<u32>(plan.availability_u32_len)?,
            output_agent_ids: runtime
                .allocate_device_buffer::<u32>(plan.output_candidate_u32_len)?,
            output_effective_distances: runtime
                .allocate_device_buffer::<f32>(plan.output_effective_f32_len)?,
            output_base_distances: runtime
                .allocate_device_buffer::<f32>(plan.output_base_f32_len)?,
            output_flags: runtime.allocate_device_buffer::<u32>(plan.output_flag_u32_len)?,
        })
    }
}

impl RouterBackend for CudaRouter {
    fn name(&self) -> &str {
        &self.name
    }

    fn route_batch(
        &self,
        requests: &[RoutingRequest],
        states: &[AgentRuntimeState],
    ) -> Result<Vec<RoutingResult>, RouteError> {
        let route_table = self.route_table.as_ref().map_err(Clone::clone)?;
        if requests.is_empty() {
            return Ok(Vec::new());
        }

        #[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
        {
            validate_k_one_inputs(route_table, requests, states)
                .map_err(cuda_route_execution_error_to_route_error)?;
            let runtime = CudaRuntime::initialize().map_err(cuda_runtime_error_to_route_error)?;
            execute_route_agents_k1(&runtime, route_table, self.coefficients, requests, states)
                .map_err(cuda_route_execution_error_to_route_error)
        }
        #[cfg(not(all(feature = "cuda-runtime", any(windows, target_os = "linux"))))]
        {
            let _ = route_table;
            let _ = requests;
            let _ = states;
            Err(RouteError::BackendUnavailable {
                backend: CUDA_BACKEND_NAME,
                reason: runtime_unavailable_error().status().reason,
            })
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CudaBackendStatus {
    pub available: bool,
    pub reason: &'static str,
    pub runtime: CudaRuntimeStatus,
}

impl CudaBackendStatus {
    pub fn unavailable(reason: &'static str) -> Self {
        Self {
            available: false,
            reason,
            runtime: detect_cuda_runtime(),
        }
    }

    pub fn from_runtime(runtime: CudaRuntimeStatus) -> Self {
        Self {
            available: false,
            reason: SCAFFOLD_REASON,
            runtime,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CudaRuntimeStatus {
    pub available: bool,
    pub device_count: usize,
    pub reason: &'static str,
    pub error_code: Option<i32>,
}

impl CudaRuntimeStatus {
    pub fn available(device_count: usize) -> Self {
        Self {
            available: true,
            device_count,
            reason: CUDA_RUNTIME_AVAILABLE_REASON,
            error_code: None,
        }
    }

    pub fn unavailable(reason: &'static str, error_code: Option<i32>) -> Self {
        Self {
            available: false,
            device_count: 0,
            reason,
            error_code,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CudaRuntimeError {
    FeatureDisabled,
    UnsupportedPlatform,
    DriverLibraryNotFound,
    MissingSymbol(&'static str),
    DriverInitFailed(i32),
    DeviceCountFailed(i32),
    NoDevice,
    DeviceGetFailed(i32),
    PrimaryContextRetainFailed(i32),
    ContextSetCurrentFailed(i32),
    DeviceAllocFailed(i32),
    DeviceFreeFailed(i32),
    HostToDeviceCopyFailed(i32),
    DeviceToHostCopyFailed(i32),
    StreamCreateFailed(i32),
    StreamDestroyFailed(i32),
    StreamSynchronizeFailed(i32),
    EventCreateFailed(i32),
    EventDestroyFailed(i32),
    EventRecordFailed(i32),
    EventSynchronizeFailed(i32),
    EventElapsedTimeFailed(i32),
    ModuleImageContainsNul,
    ModuleLoadFailed(i32),
    ModuleUnloadFailed(i32),
    FunctionNameContainsNul,
    FunctionLookupFailed(i32),
    InvalidLaunchConfig(&'static str),
    KernelLaunchFailed(i32),
    BufferLengthMismatch {
        buffer: &'static str,
        expected: usize,
        actual: usize,
    },
    ResourceSizeOverflow(&'static str),
}

impl CudaRuntimeError {
    pub fn status(self) -> CudaRuntimeStatus {
        match self {
            Self::FeatureDisabled => {
                CudaRuntimeStatus::unavailable(CUDA_RUNTIME_FEATURE_DISABLED_REASON, None)
            }
            Self::UnsupportedPlatform => {
                CudaRuntimeStatus::unavailable(CUDA_RUNTIME_UNSUPPORTED_REASON, None)
            }
            Self::DriverLibraryNotFound => {
                CudaRuntimeStatus::unavailable(CUDA_DRIVER_NOT_FOUND_REASON, None)
            }
            Self::MissingSymbol(_) => {
                CudaRuntimeStatus::unavailable(CUDA_DRIVER_SYMBOL_MISSING_REASON, None)
            }
            Self::DriverInitFailed(code) => {
                CudaRuntimeStatus::unavailable(CUDA_DRIVER_INIT_FAILED_REASON, Some(code))
            }
            Self::DeviceCountFailed(code) => {
                CudaRuntimeStatus::unavailable(CUDA_DEVICE_COUNT_FAILED_REASON, Some(code))
            }
            Self::NoDevice => CudaRuntimeStatus::unavailable(CUDA_NO_DEVICE_REASON, None),
            Self::DeviceGetFailed(code) => {
                CudaRuntimeStatus::unavailable(CUDA_DEVICE_GET_FAILED_REASON, Some(code))
            }
            Self::PrimaryContextRetainFailed(code) => {
                CudaRuntimeStatus::unavailable(CUDA_CONTEXT_RETAIN_FAILED_REASON, Some(code))
            }
            Self::ContextSetCurrentFailed(code) => {
                CudaRuntimeStatus::unavailable(CUDA_CONTEXT_SET_CURRENT_FAILED_REASON, Some(code))
            }
            Self::DeviceAllocFailed(code) => {
                CudaRuntimeStatus::unavailable(CUDA_DEVICE_ALLOC_FAILED_REASON, Some(code))
            }
            Self::DeviceFreeFailed(code) => {
                CudaRuntimeStatus::unavailable(CUDA_DEVICE_FREE_FAILED_REASON, Some(code))
            }
            Self::HostToDeviceCopyFailed(code) => {
                CudaRuntimeStatus::unavailable(CUDA_HOST_TO_DEVICE_COPY_FAILED_REASON, Some(code))
            }
            Self::DeviceToHostCopyFailed(code) => {
                CudaRuntimeStatus::unavailable(CUDA_DEVICE_TO_HOST_COPY_FAILED_REASON, Some(code))
            }
            Self::StreamCreateFailed(code) => {
                CudaRuntimeStatus::unavailable(CUDA_STREAM_CREATE_FAILED_REASON, Some(code))
            }
            Self::StreamDestroyFailed(code) => {
                CudaRuntimeStatus::unavailable(CUDA_STREAM_DESTROY_FAILED_REASON, Some(code))
            }
            Self::StreamSynchronizeFailed(code) => {
                CudaRuntimeStatus::unavailable(CUDA_STREAM_SYNCHRONIZE_FAILED_REASON, Some(code))
            }
            Self::EventCreateFailed(code) => {
                CudaRuntimeStatus::unavailable(CUDA_EVENT_CREATE_FAILED_REASON, Some(code))
            }
            Self::EventDestroyFailed(code) => {
                CudaRuntimeStatus::unavailable(CUDA_EVENT_DESTROY_FAILED_REASON, Some(code))
            }
            Self::EventRecordFailed(code) => {
                CudaRuntimeStatus::unavailable(CUDA_EVENT_RECORD_FAILED_REASON, Some(code))
            }
            Self::EventSynchronizeFailed(code) => {
                CudaRuntimeStatus::unavailable(CUDA_EVENT_SYNCHRONIZE_FAILED_REASON, Some(code))
            }
            Self::EventElapsedTimeFailed(code) => {
                CudaRuntimeStatus::unavailable(CUDA_EVENT_ELAPSED_TIME_FAILED_REASON, Some(code))
            }
            Self::ModuleImageContainsNul => {
                CudaRuntimeStatus::unavailable(CUDA_MODULE_IMAGE_CONTAINS_NUL_REASON, None)
            }
            Self::ModuleLoadFailed(code) => {
                CudaRuntimeStatus::unavailable(CUDA_MODULE_LOAD_FAILED_REASON, Some(code))
            }
            Self::ModuleUnloadFailed(code) => {
                CudaRuntimeStatus::unavailable(CUDA_MODULE_UNLOAD_FAILED_REASON, Some(code))
            }
            Self::FunctionNameContainsNul => {
                CudaRuntimeStatus::unavailable(CUDA_FUNCTION_NAME_CONTAINS_NUL_REASON, None)
            }
            Self::FunctionLookupFailed(code) => {
                CudaRuntimeStatus::unavailable(CUDA_FUNCTION_LOOKUP_FAILED_REASON, Some(code))
            }
            Self::InvalidLaunchConfig(_) => {
                CudaRuntimeStatus::unavailable(CUDA_INVALID_LAUNCH_CONFIG_REASON, None)
            }
            Self::KernelLaunchFailed(code) => {
                CudaRuntimeStatus::unavailable(CUDA_KERNEL_LAUNCH_FAILED_REASON, Some(code))
            }
            Self::BufferLengthMismatch { .. } => {
                CudaRuntimeStatus::unavailable(CUDA_BUFFER_LENGTH_MISMATCH_REASON, None)
            }
            Self::ResourceSizeOverflow(_) => {
                CudaRuntimeStatus::unavailable(CUDA_RESOURCE_SIZE_OVERFLOW_REASON, None)
            }
        }
    }
}

impl std::fmt::Display for CudaRuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FeatureDisabled => write!(f, "{CUDA_RUNTIME_FEATURE_DISABLED_REASON}"),
            Self::UnsupportedPlatform => write!(f, "{CUDA_RUNTIME_UNSUPPORTED_REASON}"),
            Self::DriverLibraryNotFound => write!(f, "{CUDA_DRIVER_NOT_FOUND_REASON}"),
            Self::MissingSymbol(symbol) => {
                write!(f, "{CUDA_DRIVER_SYMBOL_MISSING_REASON}: {symbol}")
            }
            Self::DriverInitFailed(code) => {
                write!(f, "{CUDA_DRIVER_INIT_FAILED_REASON}: {code}")
            }
            Self::DeviceCountFailed(code) => {
                write!(f, "{CUDA_DEVICE_COUNT_FAILED_REASON}: {code}")
            }
            Self::NoDevice => write!(f, "{CUDA_NO_DEVICE_REASON}"),
            Self::DeviceGetFailed(code) => {
                write!(f, "{CUDA_DEVICE_GET_FAILED_REASON}: {code}")
            }
            Self::PrimaryContextRetainFailed(code) => {
                write!(f, "{CUDA_CONTEXT_RETAIN_FAILED_REASON}: {code}")
            }
            Self::ContextSetCurrentFailed(code) => {
                write!(f, "{CUDA_CONTEXT_SET_CURRENT_FAILED_REASON}: {code}")
            }
            Self::DeviceAllocFailed(code) => {
                write!(f, "{CUDA_DEVICE_ALLOC_FAILED_REASON}: {code}")
            }
            Self::DeviceFreeFailed(code) => {
                write!(f, "{CUDA_DEVICE_FREE_FAILED_REASON}: {code}")
            }
            Self::HostToDeviceCopyFailed(code) => {
                write!(f, "{CUDA_HOST_TO_DEVICE_COPY_FAILED_REASON}: {code}")
            }
            Self::DeviceToHostCopyFailed(code) => {
                write!(f, "{CUDA_DEVICE_TO_HOST_COPY_FAILED_REASON}: {code}")
            }
            Self::StreamCreateFailed(code) => {
                write!(f, "{CUDA_STREAM_CREATE_FAILED_REASON}: {code}")
            }
            Self::StreamDestroyFailed(code) => {
                write!(f, "{CUDA_STREAM_DESTROY_FAILED_REASON}: {code}")
            }
            Self::StreamSynchronizeFailed(code) => {
                write!(f, "{CUDA_STREAM_SYNCHRONIZE_FAILED_REASON}: {code}")
            }
            Self::EventCreateFailed(code) => {
                write!(f, "{CUDA_EVENT_CREATE_FAILED_REASON}: {code}")
            }
            Self::EventDestroyFailed(code) => {
                write!(f, "{CUDA_EVENT_DESTROY_FAILED_REASON}: {code}")
            }
            Self::EventRecordFailed(code) => {
                write!(f, "{CUDA_EVENT_RECORD_FAILED_REASON}: {code}")
            }
            Self::EventSynchronizeFailed(code) => {
                write!(f, "{CUDA_EVENT_SYNCHRONIZE_FAILED_REASON}: {code}")
            }
            Self::EventElapsedTimeFailed(code) => {
                write!(f, "{CUDA_EVENT_ELAPSED_TIME_FAILED_REASON}: {code}")
            }
            Self::ModuleImageContainsNul => write!(f, "{CUDA_MODULE_IMAGE_CONTAINS_NUL_REASON}"),
            Self::ModuleLoadFailed(code) => write!(f, "{CUDA_MODULE_LOAD_FAILED_REASON}: {code}"),
            Self::ModuleUnloadFailed(code) => {
                write!(f, "{CUDA_MODULE_UNLOAD_FAILED_REASON}: {code}")
            }
            Self::FunctionNameContainsNul => {
                write!(f, "{CUDA_FUNCTION_NAME_CONTAINS_NUL_REASON}")
            }
            Self::FunctionLookupFailed(code) => {
                write!(f, "{CUDA_FUNCTION_LOOKUP_FAILED_REASON}: {code}")
            }
            Self::InvalidLaunchConfig(context) => {
                write!(f, "{CUDA_INVALID_LAUNCH_CONFIG_REASON}: {context}")
            }
            Self::KernelLaunchFailed(code) => {
                write!(f, "{CUDA_KERNEL_LAUNCH_FAILED_REASON}: {code}")
            }
            Self::BufferLengthMismatch {
                buffer,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "{CUDA_BUFFER_LENGTH_MISMATCH_REASON}: {buffer} expected {expected}, got {actual}"
                )
            }
            Self::ResourceSizeOverflow(context) => {
                write!(f, "{CUDA_RESOURCE_SIZE_OVERFLOW_REASON}: {context}")
            }
        }
    }
}

impl std::error::Error for CudaRuntimeError {}

#[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
#[derive(Debug, PartialEq)]
#[allow(dead_code)]
enum CudaRouteExecutionError {
    Route(RouteError),
    Runtime(CudaRuntimeError),
    UnsupportedK { actual: usize },
}

#[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
impl From<RouteError> for CudaRouteExecutionError {
    fn from(error: RouteError) -> Self {
        Self::Route(error)
    }
}

#[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
impl From<CudaRuntimeError> for CudaRouteExecutionError {
    fn from(error: CudaRuntimeError) -> Self {
        Self::Runtime(error)
    }
}

#[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
fn cuda_route_execution_error_to_route_error(error: CudaRouteExecutionError) -> RouteError {
    match error {
        CudaRouteExecutionError::Route(error) => error,
        CudaRouteExecutionError::Runtime(error) => cuda_runtime_error_to_route_error(error),
        CudaRouteExecutionError::UnsupportedK { .. } => RouteError::BackendUnavailable {
            backend: CUDA_BACKEND_NAME,
            reason: CUDA_K_ONE_ONLY_REASON,
        },
    }
}

#[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
fn cuda_runtime_error_to_route_error(error: CudaRuntimeError) -> RouteError {
    RouteError::BackendUnavailable {
        backend: CUDA_BACKEND_NAME,
        reason: error.status().reason,
    }
}

pub trait CudaDeviceElement: Copy + 'static + private::Sealed {}

impl CudaDeviceElement for f32 {}
impl CudaDeviceElement for u32 {}

mod private {
    pub trait Sealed {}

    impl Sealed for f32 {}
    impl Sealed for u32 {}
}

#[derive(Debug)]
pub struct CudaRuntime {
    device_count: usize,
    #[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
    context: Option<runtime_impl::CudaContext>,
    #[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
    driver: runtime_impl::CudaDriver,
}

impl CudaRuntime {
    pub fn initialize() -> Result<Self, CudaRuntimeError> {
        runtime_impl::initialize()
    }

    pub fn device_count(&self) -> usize {
        self.device_count
    }

    pub fn create_stream(&self) -> Result<CudaStream<'_>, CudaRuntimeError> {
        CudaStream::create(self)
    }

    #[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
    fn create_event(&self) -> Result<CudaEvent<'_>, CudaRuntimeError> {
        CudaEvent::create(self)
    }

    pub fn allocate_device_buffer<T: CudaDeviceElement>(
        &self,
        len: usize,
    ) -> Result<CudaDeviceBuffer<'_, T>, CudaRuntimeError> {
        CudaDeviceBuffer::allocate(self, len)
    }

    pub fn load_module_from_ptx(&self, ptx: &str) -> Result<CudaModule<'_>, CudaRuntimeError> {
        CudaModule::load_from_ptx(self, ptx)
    }

    pub fn load_route_agents_module(&self) -> Result<CudaModule<'_>, CudaRuntimeError> {
        self.load_module_from_ptx(ROUTE_AGENTS_K1_PTX)
    }
}

impl Drop for CudaRuntime {
    fn drop(&mut self) {
        #[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
        {
            let _ = self.context.take();
        }
    }
}

#[derive(Debug)]
pub struct CudaStream<'runtime> {
    _runtime: &'runtime CudaRuntime,
    #[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
    handle: Option<runtime_impl::CudaStreamHandle>,
}

impl<'runtime> CudaStream<'runtime> {
    fn create(runtime: &'runtime CudaRuntime) -> Result<Self, CudaRuntimeError> {
        #[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
        {
            Ok(Self {
                _runtime: runtime,
                handle: Some(runtime_impl::create_stream(runtime)?),
            })
        }
        #[cfg(not(all(feature = "cuda-runtime", any(windows, target_os = "linux"))))]
        {
            let _ = runtime;
            Err(runtime_unavailable_error())
        }
    }

    pub fn synchronize(&self) -> Result<(), CudaRuntimeError> {
        #[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
        {
            if let Some(handle) = self.handle {
                runtime_impl::synchronize_stream(self._runtime, handle)
            } else {
                Ok(())
            }
        }
        #[cfg(not(all(feature = "cuda-runtime", any(windows, target_os = "linux"))))]
        {
            Err(runtime_unavailable_error())
        }
    }

    pub fn destroy(self) -> Result<(), CudaRuntimeError> {
        #[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
        {
            let mut stream = self;
            if let Some(handle) = stream.handle {
                runtime_impl::destroy_stream(stream._runtime, handle)?;
                stream.handle = None;
            }
            Ok(())
        }
        #[cfg(not(all(feature = "cuda-runtime", any(windows, target_os = "linux"))))]
        {
            let _ = self;
            Err(runtime_unavailable_error())
        }
    }
}

impl Drop for CudaStream<'_> {
    fn drop(&mut self) {
        #[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
        {
            if let Some(handle) = self.handle.take() {
                let _ = runtime_impl::destroy_stream(self._runtime, handle);
            }
        }
    }
}

#[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
#[derive(Debug)]
struct CudaEvent<'runtime> {
    runtime: &'runtime CudaRuntime,
    handle: Option<runtime_impl::CudaEventHandle>,
}

#[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
impl<'runtime> CudaEvent<'runtime> {
    fn create(runtime: &'runtime CudaRuntime) -> Result<Self, CudaRuntimeError> {
        Ok(Self {
            runtime,
            handle: Some(runtime_impl::create_event(runtime)?),
        })
    }

    fn record(&self, stream: &CudaStream<'runtime>) -> Result<(), CudaRuntimeError> {
        let event_handle = self.handle.ok_or(CudaRuntimeError::EventRecordFailed(-1))?;
        let stream_handle = stream
            .handle
            .ok_or(CudaRuntimeError::EventRecordFailed(-1))?;
        runtime_impl::record_event(self.runtime, event_handle, stream_handle)
    }

    fn synchronize(&self) -> Result<(), CudaRuntimeError> {
        if let Some(handle) = self.handle {
            runtime_impl::synchronize_event(self.runtime, handle)
        } else {
            Ok(())
        }
    }

    fn elapsed_since(&self, start: &Self) -> Result<std::time::Duration, CudaRuntimeError> {
        let start_handle = start
            .handle
            .ok_or(CudaRuntimeError::EventElapsedTimeFailed(-1))?;
        let end_handle = self
            .handle
            .ok_or(CudaRuntimeError::EventElapsedTimeFailed(-1))?;
        let elapsed_ms =
            runtime_impl::elapsed_event_time_ms(self.runtime, start_handle, end_handle)?;
        Ok(std::time::Duration::from_secs_f64(
            f64::from(elapsed_ms) / 1000.0,
        ))
    }

    fn destroy(self) -> Result<(), CudaRuntimeError> {
        let mut event = self;
        if let Some(handle) = event.handle {
            runtime_impl::destroy_event(event.runtime, handle)?;
            event.handle = None;
        }
        Ok(())
    }
}

#[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
impl Drop for CudaEvent<'_> {
    fn drop(&mut self) {
        #[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
        {
            if let Some(handle) = self.handle.take() {
                let _ = runtime_impl::destroy_event(self.runtime, handle);
            }
        }
    }
}

#[derive(Debug)]
pub struct CudaDeviceBuffer<'runtime, T: CudaDeviceElement> {
    _runtime: &'runtime CudaRuntime,
    len: usize,
    byte_len: usize,
    #[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
    ptr: Option<runtime_impl::CudaDevicePtr>,
    _element: PhantomData<T>,
}

impl<'runtime, T: CudaDeviceElement> CudaDeviceBuffer<'runtime, T> {
    fn allocate(runtime: &'runtime CudaRuntime, len: usize) -> Result<Self, CudaRuntimeError> {
        let byte_len = Self::checked_byte_len(len)?;

        #[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
        {
            let ptr = if byte_len == 0 {
                None
            } else {
                Some(runtime_impl::alloc_device(runtime, byte_len)?)
            };
            Ok(Self {
                _runtime: runtime,
                len,
                byte_len,
                ptr,
                _element: PhantomData,
            })
        }
        #[cfg(not(all(feature = "cuda-runtime", any(windows, target_os = "linux"))))]
        {
            let _ = byte_len;
            let _ = runtime;
            Err(runtime_unavailable_error())
        }
    }

    pub fn checked_byte_len(len: usize) -> Result<usize, CudaRuntimeError> {
        len.checked_mul(size_of::<T>())
            .ok_or(CudaRuntimeError::ResourceSizeOverflow(
                CUDA_DEVICE_BUFFER_CONTEXT,
            ))
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn byte_len(&self) -> usize {
        self.byte_len
    }

    pub fn copy_from(&mut self, source: &[T]) -> Result<(), CudaRuntimeError> {
        validate_buffer_len("host_to_device_source", source.len(), self.len)?;

        #[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
        {
            if self.byte_len == 0 {
                return Ok(());
            }

            let ptr = self.ptr.ok_or(CudaRuntimeError::InvalidLaunchConfig(
                "device buffer pointer is missing",
            ))?;
            runtime_impl::copy_host_to_device(
                self._runtime,
                ptr,
                source.as_ptr().cast::<c_void>(),
                self.byte_len,
            )
        }
        #[cfg(not(all(feature = "cuda-runtime", any(windows, target_os = "linux"))))]
        {
            let _ = source;
            Err(runtime_unavailable_error())
        }
    }

    pub fn copy_to(&self, destination: &mut [T]) -> Result<(), CudaRuntimeError> {
        validate_buffer_len("device_to_host_destination", destination.len(), self.len)?;

        #[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
        {
            if self.byte_len == 0 {
                return Ok(());
            }

            let ptr = self.ptr.ok_or(CudaRuntimeError::InvalidLaunchConfig(
                "device buffer pointer is missing",
            ))?;
            runtime_impl::copy_device_to_host(
                self._runtime,
                destination.as_mut_ptr().cast::<c_void>(),
                ptr,
                self.byte_len,
            )
        }
        #[cfg(not(all(feature = "cuda-runtime", any(windows, target_os = "linux"))))]
        {
            let _ = destination;
            Err(runtime_unavailable_error())
        }
    }

    #[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
    fn launch_ptr(&self) -> runtime_impl::CudaDevicePtr {
        self.ptr.map_or(0, |ptr| ptr)
    }

    pub fn free(self) -> Result<(), CudaRuntimeError> {
        #[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
        {
            let mut buffer = self;
            if let Some(ptr) = buffer.ptr {
                runtime_impl::free_device(buffer._runtime, ptr)?;
                buffer.ptr = None;
            }
            Ok(())
        }
        #[cfg(not(all(feature = "cuda-runtime", any(windows, target_os = "linux"))))]
        {
            let _ = self;
            Err(runtime_unavailable_error())
        }
    }
}

impl<T: CudaDeviceElement> Drop for CudaDeviceBuffer<'_, T> {
    fn drop(&mut self) {
        #[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
        {
            if let Some(ptr) = self.ptr.take() {
                let _ = runtime_impl::free_device(self._runtime, ptr);
            }
        }
    }
}

#[derive(Debug)]
pub struct CudaModule<'runtime> {
    _runtime: &'runtime CudaRuntime,
    #[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
    handle: Option<runtime_impl::CudaModuleHandle>,
}

impl<'runtime> CudaModule<'runtime> {
    fn load_from_ptx(runtime: &'runtime CudaRuntime, ptx: &str) -> Result<Self, CudaRuntimeError> {
        let module_image =
            CString::new(ptx).map_err(|_| CudaRuntimeError::ModuleImageContainsNul)?;

        #[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
        {
            Ok(Self {
                _runtime: runtime,
                handle: Some(runtime_impl::load_module_from_image(
                    runtime,
                    module_image.as_c_str(),
                )?),
            })
        }
        #[cfg(not(all(feature = "cuda-runtime", any(windows, target_os = "linux"))))]
        {
            let _ = runtime;
            let _ = module_image;
            Err(runtime_unavailable_error())
        }
    }

    pub fn get_function<'module>(
        &'module self,
        name: &str,
    ) -> Result<CudaFunction<'module, 'runtime>, CudaRuntimeError> {
        let function_name =
            CString::new(name).map_err(|_| CudaRuntimeError::FunctionNameContainsNul)?;

        #[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
        {
            let handle = self.handle.expect("CUDA module should own a loaded handle");
            Ok(CudaFunction {
                _module: self,
                name: name.to_string(),
                _handle: runtime_impl::get_module_function(
                    self._runtime,
                    handle,
                    function_name.as_c_str(),
                )?,
            })
        }
        #[cfg(not(all(feature = "cuda-runtime", any(windows, target_os = "linux"))))]
        {
            let _ = function_name;
            Err(runtime_unavailable_error())
        }
    }

    pub fn route_agents_kernel<'module>(
        &'module self,
    ) -> Result<RouteAgentsKernel<'module, 'runtime>, CudaRuntimeError> {
        Ok(RouteAgentsKernel {
            function: self.get_function(ROUTE_AGENTS_K1_KERNEL_NAME)?,
        })
    }

    pub fn unload(self) -> Result<(), CudaRuntimeError> {
        #[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
        {
            let mut module = self;
            if let Some(handle) = module.handle {
                runtime_impl::unload_module(module._runtime, handle)?;
                module.handle = None;
            }
            Ok(())
        }
        #[cfg(not(all(feature = "cuda-runtime", any(windows, target_os = "linux"))))]
        {
            let _ = self;
            Err(runtime_unavailable_error())
        }
    }
}

impl Drop for CudaModule<'_> {
    fn drop(&mut self) {
        #[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
        {
            if let Some(handle) = self.handle.take() {
                let _ = runtime_impl::unload_module(self._runtime, handle);
            }
        }
    }
}

#[derive(Debug)]
pub struct CudaFunction<'module, 'runtime> {
    _module: &'module CudaModule<'runtime>,
    name: String,
    #[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
    _handle: runtime_impl::CudaFunctionHandle,
}

impl CudaFunction<'_, '_> {
    pub fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Debug)]
pub struct RouteAgentsKernel<'module, 'runtime> {
    function: CudaFunction<'module, 'runtime>,
}

impl<'runtime> RouteAgentsKernel<'_, 'runtime> {
    pub fn name(&self) -> &str {
        self.function.name()
    }

    pub fn launch_and_synchronize(
        &self,
        stream: &CudaStream<'runtime>,
        args: &mut RouteAgentsKernelArgs<'_, 'runtime>,
    ) -> Result<(), CudaRuntimeError> {
        self.launch(stream, args)?;
        stream.synchronize()
    }

    fn launch(
        &self,
        stream: &CudaStream<'runtime>,
        args: &mut RouteAgentsKernelArgs<'_, 'runtime>,
    ) -> Result<(), CudaRuntimeError> {
        let config = args.launch_config()?;

        #[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
        {
            let stream_handle = stream
                .handle
                .ok_or(CudaRuntimeError::InvalidLaunchConfig("stream is destroyed"))?;
            let mut pack = args.parameter_pack();
            let mut kernel_params = pack.kernel_params();

            runtime_impl::launch_kernel(
                stream._runtime,
                self.function._handle,
                stream_handle,
                config,
                &mut kernel_params,
            )
        }
        #[cfg(not(all(feature = "cuda-runtime", any(windows, target_os = "linux"))))]
        {
            let _ = stream;
            let _ = config;
            Err(runtime_unavailable_error())
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CudaKernelLaunchConfig {
    grid_dim_x: u32,
    grid_dim_y: u32,
    grid_dim_z: u32,
    block_dim_x: u32,
    block_dim_y: u32,
    block_dim_z: u32,
    shared_mem_bytes: u32,
}

impl CudaKernelLaunchConfig {
    pub fn for_1d_thread_count(
        thread_count: u32,
        block_dim_x: u32,
    ) -> Result<Self, CudaRuntimeError> {
        if block_dim_x == 0 {
            return Err(CudaRuntimeError::InvalidLaunchConfig("block_dim_x"));
        }

        Ok(Self {
            grid_dim_x: thread_count.div_ceil(block_dim_x).max(1),
            grid_dim_y: 1,
            grid_dim_z: 1,
            block_dim_x,
            block_dim_y: 1,
            block_dim_z: 1,
            shared_mem_bytes: 0,
        })
    }

    pub fn grid_dim_x(self) -> u32 {
        self.grid_dim_x
    }

    pub fn grid_dim_y(self) -> u32 {
        self.grid_dim_y
    }

    pub fn grid_dim_z(self) -> u32 {
        self.grid_dim_z
    }

    pub fn block_dim_x(self) -> u32 {
        self.block_dim_x
    }

    pub fn block_dim_y(self) -> u32 {
        self.block_dim_y
    }

    pub fn block_dim_z(self) -> u32 {
        self.block_dim_z
    }

    pub fn shared_mem_bytes(self) -> u32 {
        self.shared_mem_bytes
    }
}

#[derive(Debug)]
#[cfg_attr(
    not(all(feature = "cuda-runtime", any(windows, target_os = "linux"))),
    allow(dead_code)
)]
pub struct RouteAgentsKernelArgs<'buffers, 'runtime> {
    agent_vectors: &'buffers CudaDeviceBuffer<'runtime, f32>,
    agent_ids: &'buffers CudaDeviceBuffer<'runtime, u32>,
    request_vectors: &'buffers CudaDeviceBuffer<'runtime, f32>,
    agent_score_weights: &'buffers CudaDeviceBuffer<'runtime, f32>,
    availability: &'buffers CudaDeviceBuffer<'runtime, u32>,
    output_agent_ids: &'buffers mut CudaDeviceBuffer<'runtime, u32>,
    output_effective_distances: &'buffers mut CudaDeviceBuffer<'runtime, f32>,
    output_base_distances: &'buffers mut CudaDeviceBuffer<'runtime, f32>,
    output_flags: &'buffers mut CudaDeviceBuffer<'runtime, u32>,
    agent_count: u32,
    request_count: u32,
    dimensions: u32,
}

impl<'buffers, 'runtime> RouteAgentsKernelArgs<'buffers, 'runtime> {
    pub fn new(
        plan: CudaBufferPlan,
        agent_vectors: &'buffers CudaDeviceBuffer<'runtime, f32>,
        agent_ids: &'buffers CudaDeviceBuffer<'runtime, u32>,
        request_vectors: &'buffers CudaDeviceBuffer<'runtime, f32>,
        agent_score_weights: &'buffers CudaDeviceBuffer<'runtime, f32>,
        availability: &'buffers CudaDeviceBuffer<'runtime, u32>,
        output_agent_ids: &'buffers mut CudaDeviceBuffer<'runtime, u32>,
        output_effective_distances: &'buffers mut CudaDeviceBuffer<'runtime, f32>,
        output_base_distances: &'buffers mut CudaDeviceBuffer<'runtime, f32>,
        output_flags: &'buffers mut CudaDeviceBuffer<'runtime, u32>,
    ) -> Result<Self, CudaRuntimeError> {
        if plan.k != 1 {
            return Err(CudaRuntimeError::InvalidLaunchConfig(
                "route agents kernel currently supports k=1",
            ));
        }

        validate_buffer_len(
            "agent_vectors",
            agent_vectors.len(),
            plan.agent_vector_f32_len,
        )?;
        validate_buffer_len("agent_ids", agent_ids.len(), plan.agent_id_u32_len)?;
        validate_buffer_len(
            "request_vectors",
            request_vectors.len(),
            plan.request_vector_f32_len,
        )?;
        validate_buffer_len(
            "agent_score_weights",
            agent_score_weights.len(),
            plan.agent_score_weight_f32_len,
        )?;
        validate_buffer_len(
            "availability",
            availability.len(),
            plan.availability_u32_len,
        )?;
        validate_buffer_len(
            "output_agent_ids",
            output_agent_ids.len(),
            plan.output_candidate_u32_len,
        )?;
        validate_buffer_len(
            "output_effective_distances",
            output_effective_distances.len(),
            plan.output_effective_f32_len,
        )?;
        validate_buffer_len(
            "output_base_distances",
            output_base_distances.len(),
            plan.output_base_f32_len,
        )?;
        validate_buffer_len("output_flags", output_flags.len(), plan.output_flag_u32_len)?;

        Ok(Self {
            agent_vectors,
            agent_ids,
            request_vectors,
            agent_score_weights,
            availability,
            output_agent_ids,
            output_effective_distances,
            output_base_distances,
            output_flags,
            agent_count: checked_u32(plan.agent_count, CUDA_ROUTE_AGENTS_LAUNCH_CONTEXT)?,
            request_count: checked_u32(plan.request_count, CUDA_ROUTE_AGENTS_LAUNCH_CONTEXT)?,
            dimensions: checked_u32(plan.dimensions, CUDA_ROUTE_AGENTS_LAUNCH_CONTEXT)?,
        })
    }

    pub fn agent_count(&self) -> u32 {
        self.agent_count
    }

    pub fn request_count(&self) -> u32 {
        self.request_count
    }

    pub fn dimensions(&self) -> u32 {
        self.dimensions
    }

    pub fn launch_config(&self) -> Result<CudaKernelLaunchConfig, CudaRuntimeError> {
        CudaKernelLaunchConfig::for_1d_thread_count(self.request_count, ROUTE_AGENTS_K1_BLOCK_DIM_X)
    }

    #[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
    fn parameter_pack(&self) -> RouteAgentsKernelParameterPack {
        RouteAgentsKernelParameterPack {
            agent_vectors: self.agent_vectors.launch_ptr(),
            agent_ids: self.agent_ids.launch_ptr(),
            request_vectors: self.request_vectors.launch_ptr(),
            agent_score_weights: self.agent_score_weights.launch_ptr(),
            availability: self.availability.launch_ptr(),
            output_agent_ids: self.output_agent_ids.launch_ptr(),
            output_effective_distances: self.output_effective_distances.launch_ptr(),
            output_base_distances: self.output_base_distances.launch_ptr(),
            output_flags: self.output_flags.launch_ptr(),
            agent_count: self.agent_count,
            request_count: self.request_count,
            dimensions: self.dimensions,
        }
    }
}

#[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
#[derive(Debug)]
struct RouteAgentsKernelParameterPack {
    agent_vectors: runtime_impl::CudaDevicePtr,
    agent_ids: runtime_impl::CudaDevicePtr,
    request_vectors: runtime_impl::CudaDevicePtr,
    agent_score_weights: runtime_impl::CudaDevicePtr,
    availability: runtime_impl::CudaDevicePtr,
    output_agent_ids: runtime_impl::CudaDevicePtr,
    output_effective_distances: runtime_impl::CudaDevicePtr,
    output_base_distances: runtime_impl::CudaDevicePtr,
    output_flags: runtime_impl::CudaDevicePtr,
    agent_count: u32,
    request_count: u32,
    dimensions: u32,
}

#[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
impl RouteAgentsKernelParameterPack {
    fn kernel_params(&mut self) -> [*mut c_void; 12] {
        [
            kernel_param(&mut self.agent_vectors),
            kernel_param(&mut self.agent_ids),
            kernel_param(&mut self.request_vectors),
            kernel_param(&mut self.agent_score_weights),
            kernel_param(&mut self.availability),
            kernel_param(&mut self.output_agent_ids),
            kernel_param(&mut self.output_effective_distances),
            kernel_param(&mut self.output_base_distances),
            kernel_param(&mut self.output_flags),
            kernel_param(&mut self.agent_count),
            kernel_param(&mut self.request_count),
            kernel_param(&mut self.dimensions),
        ]
    }
}

#[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
fn kernel_param<T>(value: &mut T) -> *mut c_void {
    (value as *mut T).cast::<c_void>()
}

fn checked_u32(value: usize, context: &'static str) -> Result<u32, CudaRuntimeError> {
    value
        .try_into()
        .map_err(|_| CudaRuntimeError::ResourceSizeOverflow(context))
}

fn validate_buffer_len(
    buffer: &'static str,
    actual: usize,
    expected: usize,
) -> Result<(), CudaRuntimeError> {
    if actual == expected {
        Ok(())
    } else {
        Err(CudaRuntimeError::BufferLengthMismatch {
            buffer,
            expected,
            actual,
        })
    }
}

#[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
#[allow(dead_code)]
fn execute_route_agents_k1(
    runtime: &CudaRuntime,
    route_table: &AgentRouteTable,
    coefficients: ScoreCoefficients,
    requests: &[RoutingRequest],
    states: &[AgentRuntimeState],
) -> Result<Vec<RoutingResult>, CudaRouteExecutionError> {
    execute_route_agents_k1_timed(runtime, route_table, coefficients, requests, states)
        .map(|timed| timed.results)
}

#[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
#[allow(dead_code)]
fn execute_route_agents_k1_timed(
    runtime: &CudaRuntime,
    route_table: &AgentRouteTable,
    coefficients: ScoreCoefficients,
    requests: &[RoutingRequest],
    states: &[AgentRuntimeState],
) -> Result<CudaTimedRoutingResult, CudaRouteExecutionError> {
    let module_load_start = std::time::Instant::now();
    let module = runtime.load_route_agents_module()?;
    let module_load = module_load_start.elapsed();
    let mut timed = execute_route_agents_k1_timed_with_loaded_module(
        runtime,
        &module,
        route_table,
        coefficients,
        requests,
        states,
    )?;
    timed.timing.module_stream_setup += module_load;

    let module_unload_start = std::time::Instant::now();
    module.unload()?;
    timed.timing.module_stream_setup += module_unload_start.elapsed();

    Ok(timed)
}

#[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
#[allow(dead_code)]
fn execute_route_agents_k1_timed_with_loaded_module(
    runtime: &CudaRuntime,
    module: &CudaModule<'_>,
    route_table: &AgentRouteTable,
    coefficients: ScoreCoefficients,
    requests: &[RoutingRequest],
    states: &[AgentRuntimeState],
) -> Result<CudaTimedRoutingResult, CudaRouteExecutionError> {
    let total_start = std::time::Instant::now();
    let mut timing = CudaRouteTimingBreakdown::default();

    validate_k_one_inputs(route_table, requests, states)?;

    let host_prepare_start = std::time::Instant::now();
    let plan = CudaBufferPlan::try_new(
        route_table.len(),
        requests.len(),
        route_table.dimensions(),
        1,
    )?;
    let inputs = CudaRouteHostInputs::from_requests_and_states(requests, states, coefficients);
    timing.host_prepare = host_prepare_start.elapsed();

    let device_allocate_start = std::time::Instant::now();
    let mut workspace = CudaRouteK1Workspace::new(runtime, plan)?;
    timing.device_allocate = device_allocate_start.elapsed();

    execute_route_agents_k1_timed_with_prepared_workspace(
        runtime,
        module,
        &mut workspace,
        route_table,
        coefficients,
        requests,
        states,
        &inputs,
        timing,
        total_start,
    )
}

#[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
#[allow(dead_code)]
fn execute_route_agents_k1_timed_with_workspace<'runtime>(
    runtime: &'runtime CudaRuntime,
    module: &CudaModule<'runtime>,
    workspace: &mut CudaRouteK1Workspace<'runtime>,
    route_table: &AgentRouteTable,
    coefficients: ScoreCoefficients,
    requests: &[RoutingRequest],
    states: &[AgentRuntimeState],
) -> Result<CudaTimedRoutingResult, CudaRouteExecutionError> {
    let total_start = std::time::Instant::now();
    let mut timing = CudaRouteTimingBreakdown::default();

    validate_k_one_inputs(route_table, requests, states)?;

    let host_prepare_start = std::time::Instant::now();
    let plan = CudaBufferPlan::try_new(
        route_table.len(),
        requests.len(),
        route_table.dimensions(),
        1,
    )?;
    if workspace.plan != plan {
        return Err(CudaRuntimeError::InvalidLaunchConfig("workspace plan mismatch").into());
    }
    let inputs = CudaRouteHostInputs::from_requests_and_states(requests, states, coefficients);
    timing.host_prepare = host_prepare_start.elapsed();

    execute_route_agents_k1_timed_with_prepared_workspace(
        runtime,
        module,
        workspace,
        route_table,
        coefficients,
        requests,
        states,
        &inputs,
        timing,
        total_start,
    )
}

#[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
#[allow(clippy::too_many_arguments)]
fn execute_route_agents_k1_timed_with_prepared_workspace<'runtime>(
    runtime: &'runtime CudaRuntime,
    module: &CudaModule<'runtime>,
    workspace: &mut CudaRouteK1Workspace<'runtime>,
    route_table: &AgentRouteTable,
    coefficients: ScoreCoefficients,
    requests: &[RoutingRequest],
    states: &[AgentRuntimeState],
    inputs: &CudaRouteHostInputs,
    mut timing: CudaRouteTimingBreakdown,
    total_start: std::time::Instant,
) -> Result<CudaTimedRoutingResult, CudaRouteExecutionError> {
    let plan = workspace.plan;

    let host_to_device_start = std::time::Instant::now();
    workspace
        .agent_vectors
        .copy_from(route_table.packed_vectors())?;
    workspace.agent_ids.copy_from(route_table.agent_ids())?;
    workspace
        .request_vectors
        .copy_from(&inputs.request_vectors)?;
    workspace
        .agent_score_weights
        .copy_from(&inputs.agent_score_weights)?;
    workspace.availability.copy_from(&inputs.availability)?;
    timing.host_to_device = host_to_device_start.elapsed();

    let module_stream_setup_start = std::time::Instant::now();
    let stream = runtime.create_stream()?;
    let kernel_start_event = runtime.create_event()?;
    let kernel_stop_event = runtime.create_event()?;
    let kernel = module.route_agents_kernel()?;
    timing.module_stream_setup = module_stream_setup_start.elapsed();
    {
        let mut args = RouteAgentsKernelArgs::new(
            plan,
            &workspace.agent_vectors,
            &workspace.agent_ids,
            &workspace.request_vectors,
            &workspace.agent_score_weights,
            &workspace.availability,
            &mut workspace.output_agent_ids,
            &mut workspace.output_effective_distances,
            &mut workspace.output_base_distances,
            &mut workspace.output_flags,
        )?;
        let kernel_launch_sync_start = std::time::Instant::now();
        kernel_start_event.record(&stream)?;
        kernel.launch(&stream, &mut args)?;
        kernel_stop_event.record(&stream)?;
        kernel_stop_event.synchronize()?;
        timing.kernel_device = kernel_stop_event.elapsed_since(&kernel_start_event)?;
        timing.kernel_launch_sync = kernel_launch_sync_start.elapsed();
    }
    kernel_stop_event.destroy()?;
    kernel_start_event.destroy()?;
    stream.destroy()?;

    let device_to_host_start = std::time::Instant::now();
    let mut output = CudaRouteKernelOutput::new(plan);
    workspace.output_agent_ids.copy_to(&mut output.agent_ids)?;
    workspace
        .output_effective_distances
        .copy_to(&mut output.effective_distances)?;
    workspace
        .output_base_distances
        .copy_to(&mut output.base_distances)?;
    workspace.output_flags.copy_to(&mut output.flags)?;
    timing.device_to_host = device_to_host_start.elapsed();

    let decode_start = std::time::Instant::now();
    let results = decode_k_one_results(route_table, coefficients, requests, states, &output);
    timing.decode = decode_start.elapsed();
    timing.total = total_start.elapsed();

    Ok(CudaTimedRoutingResult { results, timing })
}

#[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
#[allow(dead_code)]
fn validate_k_one_inputs(
    route_table: &AgentRouteTable,
    requests: &[RoutingRequest],
    states: &[AgentRuntimeState],
) -> Result<(), CudaRouteExecutionError> {
    if route_table.is_empty() {
        return Err(RouteError::EmptyAgents.into());
    }
    if route_table.len() != states.len() {
        return Err(RouteError::StateLengthMismatch {
            agents: route_table.len(),
            states: states.len(),
        }
        .into());
    }

    let expected = route_table.dimensions();
    for request in requests {
        if request.k != 1 {
            return Err(CudaRouteExecutionError::UnsupportedK { actual: request.k });
        }
        if request.vector.len() != expected {
            return Err(RouteError::DimensionMismatch {
                expected,
                actual: request.vector.len(),
                context: "routing request",
            }
            .into());
        }
    }

    Ok(())
}

#[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
#[derive(Debug)]
#[allow(dead_code)]
struct CudaRouteHostInputs {
    request_vectors: Vec<f32>,
    agent_score_weights: Vec<f32>,
    availability: Vec<u32>,
}

#[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
impl CudaRouteHostInputs {
    #[allow(dead_code)]
    fn from_requests_and_states(
        requests: &[RoutingRequest],
        states: &[AgentRuntimeState],
        coefficients: ScoreCoefficients,
    ) -> Self {
        Self {
            request_vectors: requests
                .iter()
                .flat_map(|request| request.vector.iter().copied())
                .collect(),
            agent_score_weights: states
                .iter()
                .map(|state| {
                    1.0 + coefficients.alpha_queue * state.queue_depth_norm
                        + coefficients.beta_latency * state.latency_norm
                        + coefficients.gamma_cache * state.cache_pressure_norm
                })
                .collect(),
            availability: states.iter().map(|state| state.availability).collect(),
        }
    }
}

#[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
#[derive(Debug)]
#[allow(dead_code)]
struct CudaRouteKernelOutput {
    agent_ids: Vec<u32>,
    effective_distances: Vec<f32>,
    base_distances: Vec<f32>,
    flags: Vec<u32>,
}

#[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
impl CudaRouteKernelOutput {
    #[allow(dead_code)]
    fn new(plan: CudaBufferPlan) -> Self {
        Self {
            agent_ids: vec![0; plan.output_candidate_u32_len],
            effective_distances: vec![0.0; plan.output_effective_f32_len],
            base_distances: vec![0.0; plan.output_base_f32_len],
            flags: vec![0; plan.output_flag_u32_len],
        }
    }
}

#[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
#[allow(dead_code)]
fn decode_k_one_results(
    route_table: &AgentRouteTable,
    coefficients: ScoreCoefficients,
    requests: &[RoutingRequest],
    states: &[AgentRuntimeState],
    output: &CudaRouteKernelOutput,
) -> Vec<RoutingResult> {
    let agent_index_by_id = route_table
        .agent_ids()
        .iter()
        .copied()
        .enumerate()
        .map(|(index, agent_id)| (agent_id, index))
        .collect::<std::collections::HashMap<_, _>>();

    requests
        .iter()
        .enumerate()
        .map(|(idx, request)| {
            let agent_id = output.agent_ids[idx];
            let mut available_candidates = Vec::with_capacity(2);
            if agent_id != 0 {
                let agent_index = *agent_index_by_id
                    .get(&agent_id)
                    .expect("CUDA output agent id should exist in route table");
                available_candidates.push(decode_candidate(
                    agent_id,
                    output.effective_distances[idx],
                    output.base_distances[idx],
                    states[agent_index],
                    coefficients,
                ));
            }

            let used_fallback = available_candidates
                .first()
                .map(|candidate| candidate.base_distance > request.radius_max_threshold)
                .unwrap_or(true);
            if used_fallback {
                available_candidates.push(fallback_candidate(request.fallback_generalist_id));
            }

            RoutingResult {
                task_id: request.task_id,
                available_candidates,
                used_fallback,
                ideal_candidate_unavailable: output.flags[idx] != 0,
                debug: None,
            }
        })
        .collect()
}

#[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
#[allow(dead_code)]
fn decode_candidate(
    agent_id: u32,
    effective_distance: f32,
    base_distance: f32,
    state: AgentRuntimeState,
    coefficients: ScoreCoefficients,
) -> RouteCandidate {
    let queue_penalty = coefficients.alpha_queue * state.queue_depth_norm;
    let latency_penalty = coefficients.beta_latency * state.latency_norm;
    let cache_penalty = coefficients.gamma_cache * state.cache_pressure_norm;
    RouteCandidate {
        agent_id,
        effective_distance,
        base_distance,
        omega: 1.0 + queue_penalty + latency_penalty + cache_penalty,
        queue_penalty,
        latency_penalty,
        cache_penalty,
        available: true,
    }
}

#[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
#[allow(dead_code)]
fn fallback_candidate(agent_id: u32) -> RouteCandidate {
    RouteCandidate {
        agent_id,
        effective_distance: f32::INFINITY,
        base_distance: f32::INFINITY,
        omega: 1.0,
        queue_penalty: 0.0,
        latency_penalty: 0.0,
        cache_penalty: 0.0,
        available: true,
    }
}

#[cfg(not(feature = "cuda-runtime"))]
fn runtime_unavailable_error() -> CudaRuntimeError {
    CudaRuntimeError::FeatureDisabled
}

#[cfg(all(feature = "cuda-runtime", not(any(windows, target_os = "linux"))))]
fn runtime_unavailable_error() -> CudaRuntimeError {
    CudaRuntimeError::UnsupportedPlatform
}

pub fn detect_cuda_runtime() -> CudaRuntimeStatus {
    match runtime_impl::detect_device_count() {
        Ok(device_count) => CudaRuntimeStatus::available(device_count),
        Err(error) => error.status(),
    }
}

#[cfg(not(feature = "cuda-runtime"))]
mod runtime_impl {
    use super::*;

    pub(super) fn detect_device_count() -> Result<usize, CudaRuntimeError> {
        Err(CudaRuntimeError::FeatureDisabled)
    }

    pub(super) fn initialize() -> Result<CudaRuntime, CudaRuntimeError> {
        Err(CudaRuntimeError::FeatureDisabled)
    }
}

#[cfg(all(feature = "cuda-runtime", not(any(windows, target_os = "linux"))))]
mod runtime_impl {
    use super::*;

    pub(super) fn detect_device_count() -> Result<usize, CudaRuntimeError> {
        Err(CudaRuntimeError::UnsupportedPlatform)
    }

    pub(super) fn initialize() -> Result<CudaRuntime, CudaRuntimeError> {
        Err(CudaRuntimeError::UnsupportedPlatform)
    }
}

#[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
mod runtime_impl {
    use super::*;
    use std::ffi::{CStr, CString, c_char, c_int, c_uint, c_void};
    use std::ptr::NonNull;

    type CudaDriverResult = c_int;
    pub(super) type CudaDevicePtr = u64;
    pub(super) type CudaStreamHandle = NonNull<c_void>;
    pub(super) type CudaEventHandle = NonNull<c_void>;
    pub(super) type CudaModuleHandle = NonNull<c_void>;
    pub(super) type CudaFunctionHandle = NonNull<c_void>;

    #[cfg(windows)]
    type CuInitFn = unsafe extern "system" fn(c_uint) -> CudaDriverResult;
    #[cfg(target_os = "linux")]
    type CuInitFn = unsafe extern "C" fn(c_uint) -> CudaDriverResult;

    #[cfg(windows)]
    type CuDeviceGetCountFn = unsafe extern "system" fn(*mut c_int) -> CudaDriverResult;
    #[cfg(target_os = "linux")]
    type CuDeviceGetCountFn = unsafe extern "C" fn(*mut c_int) -> CudaDriverResult;

    #[cfg(windows)]
    type CuDeviceGetFn = unsafe extern "system" fn(*mut c_int, c_int) -> CudaDriverResult;
    #[cfg(target_os = "linux")]
    type CuDeviceGetFn = unsafe extern "C" fn(*mut c_int, c_int) -> CudaDriverResult;

    #[cfg(windows)]
    type CuDevicePrimaryCtxRetainFn =
        unsafe extern "system" fn(*mut *mut c_void, c_int) -> CudaDriverResult;
    #[cfg(target_os = "linux")]
    type CuDevicePrimaryCtxRetainFn =
        unsafe extern "C" fn(*mut *mut c_void, c_int) -> CudaDriverResult;

    #[cfg(windows)]
    type CuDevicePrimaryCtxReleaseFn = unsafe extern "system" fn(c_int) -> CudaDriverResult;
    #[cfg(target_os = "linux")]
    type CuDevicePrimaryCtxReleaseFn = unsafe extern "C" fn(c_int) -> CudaDriverResult;

    #[cfg(windows)]
    type CuCtxSetCurrentFn = unsafe extern "system" fn(*mut c_void) -> CudaDriverResult;
    #[cfg(target_os = "linux")]
    type CuCtxSetCurrentFn = unsafe extern "C" fn(*mut c_void) -> CudaDriverResult;

    #[cfg(windows)]
    type CuMemAllocFn = unsafe extern "system" fn(*mut CudaDevicePtr, usize) -> CudaDriverResult;
    #[cfg(target_os = "linux")]
    type CuMemAllocFn = unsafe extern "C" fn(*mut CudaDevicePtr, usize) -> CudaDriverResult;

    #[cfg(windows)]
    type CuMemFreeFn = unsafe extern "system" fn(CudaDevicePtr) -> CudaDriverResult;
    #[cfg(target_os = "linux")]
    type CuMemFreeFn = unsafe extern "C" fn(CudaDevicePtr) -> CudaDriverResult;

    #[cfg(windows)]
    type CuMemcpyHtoDFn =
        unsafe extern "system" fn(CudaDevicePtr, *const c_void, usize) -> CudaDriverResult;
    #[cfg(target_os = "linux")]
    type CuMemcpyHtoDFn =
        unsafe extern "C" fn(CudaDevicePtr, *const c_void, usize) -> CudaDriverResult;

    #[cfg(windows)]
    type CuMemcpyDtoHFn =
        unsafe extern "system" fn(*mut c_void, CudaDevicePtr, usize) -> CudaDriverResult;
    #[cfg(target_os = "linux")]
    type CuMemcpyDtoHFn =
        unsafe extern "C" fn(*mut c_void, CudaDevicePtr, usize) -> CudaDriverResult;

    #[cfg(windows)]
    type CuStreamCreateFn = unsafe extern "system" fn(*mut *mut c_void, c_uint) -> CudaDriverResult;
    #[cfg(target_os = "linux")]
    type CuStreamCreateFn = unsafe extern "C" fn(*mut *mut c_void, c_uint) -> CudaDriverResult;

    #[cfg(windows)]
    type CuStreamDestroyFn = unsafe extern "system" fn(*mut c_void) -> CudaDriverResult;
    #[cfg(target_os = "linux")]
    type CuStreamDestroyFn = unsafe extern "C" fn(*mut c_void) -> CudaDriverResult;

    #[cfg(windows)]
    type CuStreamSynchronizeFn = unsafe extern "system" fn(*mut c_void) -> CudaDriverResult;
    #[cfg(target_os = "linux")]
    type CuStreamSynchronizeFn = unsafe extern "C" fn(*mut c_void) -> CudaDriverResult;

    #[cfg(windows)]
    type CuEventCreateFn = unsafe extern "system" fn(*mut *mut c_void, c_uint) -> CudaDriverResult;
    #[cfg(target_os = "linux")]
    type CuEventCreateFn = unsafe extern "C" fn(*mut *mut c_void, c_uint) -> CudaDriverResult;

    #[cfg(windows)]
    type CuEventDestroyFn = unsafe extern "system" fn(*mut c_void) -> CudaDriverResult;
    #[cfg(target_os = "linux")]
    type CuEventDestroyFn = unsafe extern "C" fn(*mut c_void) -> CudaDriverResult;

    #[cfg(windows)]
    type CuEventRecordFn = unsafe extern "system" fn(*mut c_void, *mut c_void) -> CudaDriverResult;
    #[cfg(target_os = "linux")]
    type CuEventRecordFn = unsafe extern "C" fn(*mut c_void, *mut c_void) -> CudaDriverResult;

    #[cfg(windows)]
    type CuEventSynchronizeFn = unsafe extern "system" fn(*mut c_void) -> CudaDriverResult;
    #[cfg(target_os = "linux")]
    type CuEventSynchronizeFn = unsafe extern "C" fn(*mut c_void) -> CudaDriverResult;

    #[cfg(windows)]
    type CuEventElapsedTimeFn =
        unsafe extern "system" fn(*mut f32, *mut c_void, *mut c_void) -> CudaDriverResult;
    #[cfg(target_os = "linux")]
    type CuEventElapsedTimeFn =
        unsafe extern "C" fn(*mut f32, *mut c_void, *mut c_void) -> CudaDriverResult;

    #[cfg(windows)]
    type CuModuleLoadDataFn =
        unsafe extern "system" fn(*mut *mut c_void, *const c_void) -> CudaDriverResult;
    #[cfg(target_os = "linux")]
    type CuModuleLoadDataFn =
        unsafe extern "C" fn(*mut *mut c_void, *const c_void) -> CudaDriverResult;

    #[cfg(windows)]
    type CuModuleUnloadFn = unsafe extern "system" fn(*mut c_void) -> CudaDriverResult;
    #[cfg(target_os = "linux")]
    type CuModuleUnloadFn = unsafe extern "C" fn(*mut c_void) -> CudaDriverResult;

    #[cfg(windows)]
    type CuModuleGetFunctionFn =
        unsafe extern "system" fn(*mut *mut c_void, *mut c_void, *const c_char) -> CudaDriverResult;
    #[cfg(target_os = "linux")]
    type CuModuleGetFunctionFn =
        unsafe extern "C" fn(*mut *mut c_void, *mut c_void, *const c_char) -> CudaDriverResult;

    #[cfg(windows)]
    type CuLaunchKernelFn = unsafe extern "system" fn(
        *mut c_void,
        c_uint,
        c_uint,
        c_uint,
        c_uint,
        c_uint,
        c_uint,
        c_uint,
        *mut c_void,
        *mut *mut c_void,
        *mut *mut c_void,
    ) -> CudaDriverResult;
    #[cfg(target_os = "linux")]
    type CuLaunchKernelFn = unsafe extern "C" fn(
        *mut c_void,
        c_uint,
        c_uint,
        c_uint,
        c_uint,
        c_uint,
        c_uint,
        c_uint,
        *mut c_void,
        *mut *mut c_void,
        *mut *mut c_void,
    ) -> CudaDriverResult;

    #[derive(Debug)]
    pub(super) struct CudaDriver {
        symbols: DriverSymbols,
    }

    impl CudaDriver {
        fn load() -> Result<Self, CudaRuntimeError> {
            Ok(Self {
                symbols: DriverSymbols::load()?,
            })
        }

        fn init(&self) -> Result<(), CudaRuntimeError> {
            // SAFETY: `cu_init` is loaded from the CUDA driver library under its documented ABI.
            // Passing flags=0 is the CUDA Driver API initialization contract.
            let result = unsafe { (self.symbols.cu_init)(0) };
            if result == 0 {
                Ok(())
            } else {
                Err(CudaRuntimeError::DriverInitFailed(result))
            }
        }

        fn device_count(&self) -> Result<usize, CudaRuntimeError> {
            let mut count = 0;
            // SAFETY: `cu_device_get_count` is loaded from the CUDA driver library, and `count`
            // is a valid out-pointer for the duration of the call.
            let result = unsafe { (self.symbols.cu_device_get_count)(&mut count) };
            if result != 0 {
                return Err(CudaRuntimeError::DeviceCountFailed(result));
            }
            if count <= 0 {
                return Err(CudaRuntimeError::NoDevice);
            }

            Ok(count as usize)
        }

        fn device(&self, ordinal: c_int) -> Result<c_int, CudaRuntimeError> {
            let mut device = 0;
            // SAFETY: `cu_device_get` is loaded from the CUDA driver library, and `device` is a
            // valid out-pointer for the duration of the call.
            let result = unsafe { (self.symbols.cu_device_get)(&mut device, ordinal) };
            if result == 0 {
                Ok(device)
            } else {
                Err(CudaRuntimeError::DeviceGetFailed(result))
            }
        }
    }

    #[derive(Debug)]
    pub(super) struct CudaContext {
        device: c_int,
        handle: NonNull<c_void>,
        cu_ctx_set_current: CuCtxSetCurrentFn,
        cu_device_primary_ctx_release: CuDevicePrimaryCtxReleaseFn,
    }

    impl CudaContext {
        fn retain(driver: &CudaDriver, device: c_int) -> Result<Self, CudaRuntimeError> {
            let mut context = std::ptr::null_mut();
            // SAFETY: `cu_device_primary_ctx_retain` is loaded from the CUDA driver library,
            // `context` is a valid out-pointer, and `device` came from `cuDeviceGet`.
            let result =
                unsafe { (driver.symbols.cu_device_primary_ctx_retain)(&mut context, device) };
            if result != 0 {
                return Err(CudaRuntimeError::PrimaryContextRetainFailed(result));
            }

            let Some(handle) = NonNull::new(context) else {
                return Err(CudaRuntimeError::PrimaryContextRetainFailed(-1));
            };

            let retained = Self {
                device,
                handle,
                cu_ctx_set_current: driver.symbols.cu_ctx_set_current,
                cu_device_primary_ctx_release: driver.symbols.cu_device_primary_ctx_release,
            };
            if let Err(error) = retained.set_current() {
                // SAFETY: The primary context was retained above for this device and is released
                // here because initialization is aborting before `CudaContext` ownership escapes.
                let _ = unsafe { (retained.cu_device_primary_ctx_release)(retained.device) };
                return Err(error);
            }

            Ok(retained)
        }

        fn set_current(&self) -> Result<(), CudaRuntimeError> {
            // SAFETY: `handle` is a retained CUDA primary context and the function pointer is
            // loaded from the still-live CUDA driver library.
            let result = unsafe { (self.cu_ctx_set_current)(self.handle.as_ptr()) };
            if result == 0 {
                Ok(())
            } else {
                Err(CudaRuntimeError::ContextSetCurrentFailed(result))
            }
        }
    }

    impl Drop for CudaContext {
        fn drop(&mut self) {
            let _ = self.set_current();
            // SAFETY: This releases the primary context retained by `CudaContext::retain`.
            let _ = unsafe { (self.cu_device_primary_ctx_release)(self.device) };
        }
    }

    #[derive(Debug)]
    struct DriverSymbols {
        _library: DriverLibrary,
        cu_init: CuInitFn,
        cu_device_get_count: CuDeviceGetCountFn,
        cu_device_get: CuDeviceGetFn,
        cu_device_primary_ctx_retain: CuDevicePrimaryCtxRetainFn,
        cu_device_primary_ctx_release: CuDevicePrimaryCtxReleaseFn,
        cu_ctx_set_current: CuCtxSetCurrentFn,
        cu_mem_alloc: CuMemAllocFn,
        cu_mem_free: CuMemFreeFn,
        cu_memcpy_htod: CuMemcpyHtoDFn,
        cu_memcpy_dtoh: CuMemcpyDtoHFn,
        cu_stream_create: CuStreamCreateFn,
        cu_stream_destroy: CuStreamDestroyFn,
        cu_stream_synchronize: CuStreamSynchronizeFn,
        cu_event_create: CuEventCreateFn,
        cu_event_destroy: CuEventDestroyFn,
        cu_event_record: CuEventRecordFn,
        cu_event_synchronize: CuEventSynchronizeFn,
        cu_event_elapsed_time: CuEventElapsedTimeFn,
        cu_module_load_data: CuModuleLoadDataFn,
        cu_module_unload: CuModuleUnloadFn,
        cu_module_get_function: CuModuleGetFunctionFn,
        cu_launch_kernel: CuLaunchKernelFn,
    }

    impl DriverSymbols {
        fn load() -> Result<Self, CudaRuntimeError> {
            let library = DriverLibrary::open_any(driver_library_names())?;
            let cu_init_ptr = library.symbol_ptr(b"cuInit\0", "cuInit")?;
            let cu_device_get_count_ptr =
                library.symbol_ptr(b"cuDeviceGetCount\0", "cuDeviceGetCount")?;
            let cu_device_get_ptr = library.symbol_ptr(b"cuDeviceGet\0", "cuDeviceGet")?;
            let cu_device_primary_ctx_retain_ptr =
                library.symbol_ptr(b"cuDevicePrimaryCtxRetain\0", "cuDevicePrimaryCtxRetain")?;
            let cu_device_primary_ctx_release_ptr = library.symbol_ptr_any(
                &[
                    (
                        b"cuDevicePrimaryCtxRelease_v2\0",
                        "cuDevicePrimaryCtxRelease_v2",
                    ),
                    (b"cuDevicePrimaryCtxRelease\0", "cuDevicePrimaryCtxRelease"),
                ],
                "cuDevicePrimaryCtxRelease",
            )?;
            let cu_ctx_set_current_ptr =
                library.symbol_ptr(b"cuCtxSetCurrent\0", "cuCtxSetCurrent")?;
            let cu_mem_alloc_ptr = library.symbol_ptr_any(
                &[
                    (b"cuMemAlloc_v2\0", "cuMemAlloc_v2"),
                    (b"cuMemAlloc\0", "cuMemAlloc"),
                ],
                "cuMemAlloc",
            )?;
            let cu_mem_free_ptr = library.symbol_ptr_any(
                &[
                    (b"cuMemFree_v2\0", "cuMemFree_v2"),
                    (b"cuMemFree\0", "cuMemFree"),
                ],
                "cuMemFree",
            )?;
            let cu_memcpy_htod_ptr = library.symbol_ptr_any(
                &[
                    (b"cuMemcpyHtoD_v2\0", "cuMemcpyHtoD_v2"),
                    (b"cuMemcpyHtoD\0", "cuMemcpyHtoD"),
                ],
                "cuMemcpyHtoD",
            )?;
            let cu_memcpy_dtoh_ptr = library.symbol_ptr_any(
                &[
                    (b"cuMemcpyDtoH_v2\0", "cuMemcpyDtoH_v2"),
                    (b"cuMemcpyDtoH\0", "cuMemcpyDtoH"),
                ],
                "cuMemcpyDtoH",
            )?;
            let cu_stream_create_ptr = library.symbol_ptr(b"cuStreamCreate\0", "cuStreamCreate")?;
            let cu_stream_destroy_ptr = library.symbol_ptr_any(
                &[
                    (b"cuStreamDestroy_v2\0", "cuStreamDestroy_v2"),
                    (b"cuStreamDestroy\0", "cuStreamDestroy"),
                ],
                "cuStreamDestroy",
            )?;
            let cu_stream_synchronize_ptr =
                library.symbol_ptr(b"cuStreamSynchronize\0", "cuStreamSynchronize")?;
            let cu_event_create_ptr = library.symbol_ptr(b"cuEventCreate\0", "cuEventCreate")?;
            let cu_event_destroy_ptr = library.symbol_ptr_any(
                &[
                    (b"cuEventDestroy_v2\0", "cuEventDestroy_v2"),
                    (b"cuEventDestroy\0", "cuEventDestroy"),
                ],
                "cuEventDestroy",
            )?;
            let cu_event_record_ptr = library.symbol_ptr(b"cuEventRecord\0", "cuEventRecord")?;
            let cu_event_synchronize_ptr =
                library.symbol_ptr(b"cuEventSynchronize\0", "cuEventSynchronize")?;
            let cu_event_elapsed_time_ptr =
                library.symbol_ptr(b"cuEventElapsedTime\0", "cuEventElapsedTime")?;
            let cu_module_load_data_ptr =
                library.symbol_ptr(b"cuModuleLoadData\0", "cuModuleLoadData")?;
            let cu_module_unload_ptr = library.symbol_ptr(b"cuModuleUnload\0", "cuModuleUnload")?;
            let cu_module_get_function_ptr =
                library.symbol_ptr(b"cuModuleGetFunction\0", "cuModuleGetFunction")?;
            let cu_launch_kernel_ptr = library.symbol_ptr(b"cuLaunchKernel\0", "cuLaunchKernel")?;

            // SAFETY: The symbol is resolved by exact CUDA Driver API name and cast to its
            // documented ABI signature. The owned `DriverLibrary` keeps the module loaded.
            let cu_init = unsafe { std::mem::transmute::<*mut c_void, CuInitFn>(cu_init_ptr) };
            // SAFETY: Same invariant as `cu_init`; this symbol name maps to `cuDeviceGetCount`.
            let cu_device_get_count = unsafe {
                std::mem::transmute::<*mut c_void, CuDeviceGetCountFn>(cu_device_get_count_ptr)
            };
            // SAFETY: Same invariant as `cu_init`; this symbol name maps to `cuDeviceGet`.
            let cu_device_get =
                unsafe { std::mem::transmute::<*mut c_void, CuDeviceGetFn>(cu_device_get_ptr) };
            // SAFETY: Same invariant as `cu_init`; this symbol maps to primary context retain.
            let cu_device_primary_ctx_retain = unsafe {
                std::mem::transmute::<*mut c_void, CuDevicePrimaryCtxRetainFn>(
                    cu_device_primary_ctx_retain_ptr,
                )
            };
            // SAFETY: Same invariant as `cu_init`; this symbol maps to primary context release.
            let cu_device_primary_ctx_release = unsafe {
                std::mem::transmute::<*mut c_void, CuDevicePrimaryCtxReleaseFn>(
                    cu_device_primary_ctx_release_ptr,
                )
            };
            // SAFETY: Same invariant as `cu_init`; this symbol name maps to `cuCtxSetCurrent`.
            let cu_ctx_set_current = unsafe {
                std::mem::transmute::<*mut c_void, CuCtxSetCurrentFn>(cu_ctx_set_current_ptr)
            };
            // SAFETY: Same invariant as `cu_init`; this symbol name maps to `cuMemAlloc`.
            let cu_mem_alloc =
                unsafe { std::mem::transmute::<*mut c_void, CuMemAllocFn>(cu_mem_alloc_ptr) };
            // SAFETY: Same invariant as `cu_init`; this symbol name maps to `cuMemFree`.
            let cu_mem_free =
                unsafe { std::mem::transmute::<*mut c_void, CuMemFreeFn>(cu_mem_free_ptr) };
            // SAFETY: Same invariant as `cu_init`; this symbol name maps to `cuMemcpyHtoD`.
            let cu_memcpy_htod =
                unsafe { std::mem::transmute::<*mut c_void, CuMemcpyHtoDFn>(cu_memcpy_htod_ptr) };
            // SAFETY: Same invariant as `cu_init`; this symbol name maps to `cuMemcpyDtoH`.
            let cu_memcpy_dtoh =
                unsafe { std::mem::transmute::<*mut c_void, CuMemcpyDtoHFn>(cu_memcpy_dtoh_ptr) };
            // SAFETY: Same invariant as `cu_init`; this symbol name maps to `cuStreamCreate`.
            let cu_stream_create = unsafe {
                std::mem::transmute::<*mut c_void, CuStreamCreateFn>(cu_stream_create_ptr)
            };
            // SAFETY: Same invariant as `cu_init`; this symbol name maps to `cuStreamDestroy`.
            let cu_stream_destroy = unsafe {
                std::mem::transmute::<*mut c_void, CuStreamDestroyFn>(cu_stream_destroy_ptr)
            };
            // SAFETY: Same invariant as `cu_init`; this symbol maps to `cuStreamSynchronize`.
            let cu_stream_synchronize = unsafe {
                std::mem::transmute::<*mut c_void, CuStreamSynchronizeFn>(cu_stream_synchronize_ptr)
            };
            // SAFETY: Same invariant as `cu_init`; this symbol name maps to `cuEventCreate`.
            let cu_event_create =
                unsafe { std::mem::transmute::<*mut c_void, CuEventCreateFn>(cu_event_create_ptr) };
            // SAFETY: Same invariant as `cu_init`; this symbol name maps to `cuEventDestroy`.
            let cu_event_destroy = unsafe {
                std::mem::transmute::<*mut c_void, CuEventDestroyFn>(cu_event_destroy_ptr)
            };
            // SAFETY: Same invariant as `cu_init`; this symbol name maps to `cuEventRecord`.
            let cu_event_record =
                unsafe { std::mem::transmute::<*mut c_void, CuEventRecordFn>(cu_event_record_ptr) };
            // SAFETY: Same invariant as `cu_init`; this symbol maps to `cuEventSynchronize`.
            let cu_event_synchronize = unsafe {
                std::mem::transmute::<*mut c_void, CuEventSynchronizeFn>(cu_event_synchronize_ptr)
            };
            // SAFETY: Same invariant as `cu_init`; this symbol maps to `cuEventElapsedTime`.
            let cu_event_elapsed_time = unsafe {
                std::mem::transmute::<*mut c_void, CuEventElapsedTimeFn>(cu_event_elapsed_time_ptr)
            };
            // SAFETY: Same invariant as `cu_init`; this symbol name maps to `cuModuleLoadData`.
            let cu_module_load_data = unsafe {
                std::mem::transmute::<*mut c_void, CuModuleLoadDataFn>(cu_module_load_data_ptr)
            };
            // SAFETY: Same invariant as `cu_init`; this symbol name maps to `cuModuleUnload`.
            let cu_module_unload = unsafe {
                std::mem::transmute::<*mut c_void, CuModuleUnloadFn>(cu_module_unload_ptr)
            };
            // SAFETY: Same invariant as `cu_init`; this symbol maps to `cuModuleGetFunction`.
            let cu_module_get_function = unsafe {
                std::mem::transmute::<*mut c_void, CuModuleGetFunctionFn>(
                    cu_module_get_function_ptr,
                )
            };
            // SAFETY: Same invariant as `cu_init`; this symbol maps to `cuLaunchKernel`.
            let cu_launch_kernel = unsafe {
                std::mem::transmute::<*mut c_void, CuLaunchKernelFn>(cu_launch_kernel_ptr)
            };

            Ok(Self {
                _library: library,
                cu_init,
                cu_device_get_count,
                cu_device_get,
                cu_device_primary_ctx_retain,
                cu_device_primary_ctx_release,
                cu_ctx_set_current,
                cu_mem_alloc,
                cu_mem_free,
                cu_memcpy_htod,
                cu_memcpy_dtoh,
                cu_stream_create,
                cu_stream_destroy,
                cu_stream_synchronize,
                cu_event_create,
                cu_event_destroy,
                cu_event_record,
                cu_event_synchronize,
                cu_event_elapsed_time,
                cu_module_load_data,
                cu_module_unload,
                cu_module_get_function,
                cu_launch_kernel,
            })
        }
    }

    #[derive(Debug)]
    struct DriverLibrary {
        handle: NonNull<c_void>,
    }

    impl DriverLibrary {
        fn open_any(names: &[&str]) -> Result<Self, CudaRuntimeError> {
            for name in names {
                if let Some(library) = Self::open(name) {
                    return Ok(library);
                }
            }

            Err(CudaRuntimeError::DriverLibraryNotFound)
        }

        fn open(name: &str) -> Option<Self> {
            let name = CString::new(name).ok()?;
            let handle = platform_open(&name)?;
            Some(Self { handle })
        }

        fn symbol_ptr(
            &self,
            symbol: &'static [u8],
            label: &'static str,
        ) -> Result<*mut c_void, CudaRuntimeError> {
            let symbol =
                CStr::from_bytes_with_nul(symbol).expect("CUDA symbol literals must be nul-ended");
            platform_symbol(self.handle, symbol).ok_or(CudaRuntimeError::MissingSymbol(label))
        }

        fn symbol_ptr_any(
            &self,
            symbols: &[(&'static [u8], &'static str)],
            label: &'static str,
        ) -> Result<*mut c_void, CudaRuntimeError> {
            symbols
                .iter()
                .find_map(|(symbol, _)| {
                    let symbol = CStr::from_bytes_with_nul(symbol)
                        .expect("CUDA symbol literals must be nul-ended");
                    platform_symbol(self.handle, symbol)
                })
                .ok_or(CudaRuntimeError::MissingSymbol(label))
        }
    }

    impl Drop for DriverLibrary {
        fn drop(&mut self) {
            platform_close(self.handle);
        }
    }

    pub(super) fn detect_device_count() -> Result<usize, CudaRuntimeError> {
        let driver = CudaDriver::load()?;
        driver.init()?;
        driver.device_count()
    }

    pub(super) fn initialize() -> Result<CudaRuntime, CudaRuntimeError> {
        let driver = CudaDriver::load()?;
        driver.init()?;
        let device_count = driver.device_count()?;
        let device = driver.device(0)?;
        let context = CudaContext::retain(&driver, device)?;

        Ok(CudaRuntime {
            device_count,
            context: Some(context),
            driver,
        })
    }

    pub(super) fn alloc_device(
        runtime: &CudaRuntime,
        byte_len: usize,
    ) -> Result<CudaDevicePtr, CudaRuntimeError> {
        set_current_context(runtime)?;

        let mut ptr = 0;
        // SAFETY: A current CUDA context is active, `ptr` is a valid out-pointer, and `byte_len`
        // was checked by `CudaDeviceBuffer::checked_byte_len`.
        let result = unsafe { (runtime.driver.symbols.cu_mem_alloc)(&mut ptr, byte_len) };
        if result == 0 {
            Ok(ptr)
        } else {
            Err(CudaRuntimeError::DeviceAllocFailed(result))
        }
    }

    pub(super) fn free_device(
        runtime: &CudaRuntime,
        ptr: CudaDevicePtr,
    ) -> Result<(), CudaRuntimeError> {
        set_current_context(runtime)?;

        // SAFETY: `ptr` was returned by `cuMemAlloc` for this runtime context and is freed at
        // most once by `CudaDeviceBuffer`.
        let result = unsafe { (runtime.driver.symbols.cu_mem_free)(ptr) };
        if result == 0 {
            Ok(())
        } else {
            Err(CudaRuntimeError::DeviceFreeFailed(result))
        }
    }

    pub(super) fn copy_host_to_device(
        runtime: &CudaRuntime,
        destination: CudaDevicePtr,
        source: *const c_void,
        byte_len: usize,
    ) -> Result<(), CudaRuntimeError> {
        set_current_context(runtime)?;

        // SAFETY: `destination` is a live device allocation for this context, `source` points to
        // `byte_len` initialized host bytes, and the caller validated the typed buffer length.
        let result =
            unsafe { (runtime.driver.symbols.cu_memcpy_htod)(destination, source, byte_len) };
        if result == 0 {
            Ok(())
        } else {
            Err(CudaRuntimeError::HostToDeviceCopyFailed(result))
        }
    }

    pub(super) fn copy_device_to_host(
        runtime: &CudaRuntime,
        destination: *mut c_void,
        source: CudaDevicePtr,
        byte_len: usize,
    ) -> Result<(), CudaRuntimeError> {
        set_current_context(runtime)?;

        // SAFETY: `source` is a live device allocation for this context, `destination` points to
        // `byte_len` writable host bytes, and the caller validated the typed buffer length.
        let result =
            unsafe { (runtime.driver.symbols.cu_memcpy_dtoh)(destination, source, byte_len) };
        if result == 0 {
            Ok(())
        } else {
            Err(CudaRuntimeError::DeviceToHostCopyFailed(result))
        }
    }

    pub(super) fn create_stream(
        runtime: &CudaRuntime,
    ) -> Result<CudaStreamHandle, CudaRuntimeError> {
        set_current_context(runtime)?;

        let mut stream = std::ptr::null_mut();
        // SAFETY: A current CUDA context is active and `stream` is a valid out-pointer.
        let result = unsafe { (runtime.driver.symbols.cu_stream_create)(&mut stream, 0) };
        if result != 0 {
            return Err(CudaRuntimeError::StreamCreateFailed(result));
        }

        NonNull::new(stream).ok_or(CudaRuntimeError::StreamCreateFailed(-1))
    }

    pub(super) fn synchronize_stream(
        runtime: &CudaRuntime,
        stream: CudaStreamHandle,
    ) -> Result<(), CudaRuntimeError> {
        set_current_context(runtime)?;

        // SAFETY: `stream` was created by `cuStreamCreate` for this runtime context and remains
        // owned by `CudaStream`.
        let result = unsafe { (runtime.driver.symbols.cu_stream_synchronize)(stream.as_ptr()) };
        if result == 0 {
            Ok(())
        } else {
            Err(CudaRuntimeError::StreamSynchronizeFailed(result))
        }
    }

    pub(super) fn destroy_stream(
        runtime: &CudaRuntime,
        stream: CudaStreamHandle,
    ) -> Result<(), CudaRuntimeError> {
        set_current_context(runtime)?;

        // SAFETY: `stream` was created by `cuStreamCreate` for this runtime context and is
        // destroyed at most once by `CudaStream`.
        let result = unsafe { (runtime.driver.symbols.cu_stream_destroy)(stream.as_ptr()) };
        if result == 0 {
            Ok(())
        } else {
            Err(CudaRuntimeError::StreamDestroyFailed(result))
        }
    }

    pub(super) fn create_event(runtime: &CudaRuntime) -> Result<CudaEventHandle, CudaRuntimeError> {
        set_current_context(runtime)?;

        let mut event = std::ptr::null_mut();
        // SAFETY: A current CUDA context is active and `event` is a valid out-pointer.
        let result = unsafe { (runtime.driver.symbols.cu_event_create)(&mut event, 0) };
        if result != 0 {
            return Err(CudaRuntimeError::EventCreateFailed(result));
        }

        NonNull::new(event).ok_or(CudaRuntimeError::EventCreateFailed(-1))
    }

    pub(super) fn record_event(
        runtime: &CudaRuntime,
        event: CudaEventHandle,
        stream: CudaStreamHandle,
    ) -> Result<(), CudaRuntimeError> {
        set_current_context(runtime)?;

        // SAFETY: `event` and `stream` were created for this runtime context and remain owned by
        // safe wrappers for the duration of this call.
        let result =
            unsafe { (runtime.driver.symbols.cu_event_record)(event.as_ptr(), stream.as_ptr()) };
        if result == 0 {
            Ok(())
        } else {
            Err(CudaRuntimeError::EventRecordFailed(result))
        }
    }

    pub(super) fn synchronize_event(
        runtime: &CudaRuntime,
        event: CudaEventHandle,
    ) -> Result<(), CudaRuntimeError> {
        set_current_context(runtime)?;

        // SAFETY: `event` was created by `cuEventCreate` for this runtime context and remains
        // owned by `CudaEvent`.
        let result = unsafe { (runtime.driver.symbols.cu_event_synchronize)(event.as_ptr()) };
        if result == 0 {
            Ok(())
        } else {
            Err(CudaRuntimeError::EventSynchronizeFailed(result))
        }
    }

    pub(super) fn elapsed_event_time_ms(
        runtime: &CudaRuntime,
        start: CudaEventHandle,
        end: CudaEventHandle,
    ) -> Result<f32, CudaRuntimeError> {
        set_current_context(runtime)?;

        let mut elapsed_ms = 0.0_f32;
        // SAFETY: Both events were recorded in this context and `elapsed_ms` is a valid
        // out-pointer for CUDA to write the elapsed milliseconds.
        let result = unsafe {
            (runtime.driver.symbols.cu_event_elapsed_time)(
                &mut elapsed_ms,
                start.as_ptr(),
                end.as_ptr(),
            )
        };
        if result == 0 {
            Ok(elapsed_ms)
        } else {
            Err(CudaRuntimeError::EventElapsedTimeFailed(result))
        }
    }

    pub(super) fn destroy_event(
        runtime: &CudaRuntime,
        event: CudaEventHandle,
    ) -> Result<(), CudaRuntimeError> {
        set_current_context(runtime)?;

        // SAFETY: `event` was created by `cuEventCreate` for this runtime context and is
        // destroyed at most once by `CudaEvent`.
        let result = unsafe { (runtime.driver.symbols.cu_event_destroy)(event.as_ptr()) };
        if result == 0 {
            Ok(())
        } else {
            Err(CudaRuntimeError::EventDestroyFailed(result))
        }
    }

    pub(super) fn load_module_from_image(
        runtime: &CudaRuntime,
        module_image: &CStr,
    ) -> Result<CudaModuleHandle, CudaRuntimeError> {
        set_current_context(runtime)?;

        let mut module = std::ptr::null_mut();
        // SAFETY: A current CUDA context is active, `module` is a valid out-pointer, and
        // `module_image` is a nul-terminated PTX image that lives for the duration of the call.
        let result = unsafe {
            (runtime.driver.symbols.cu_module_load_data)(
                &mut module,
                module_image.as_ptr().cast::<c_void>(),
            )
        };
        if result != 0 {
            return Err(CudaRuntimeError::ModuleLoadFailed(result));
        }

        NonNull::new(module).ok_or(CudaRuntimeError::ModuleLoadFailed(-1))
    }

    pub(super) fn unload_module(
        runtime: &CudaRuntime,
        module: CudaModuleHandle,
    ) -> Result<(), CudaRuntimeError> {
        set_current_context(runtime)?;

        // SAFETY: `module` was returned by `cuModuleLoadData` for this runtime context and is
        // unloaded at most once by `CudaModule`.
        let result = unsafe { (runtime.driver.symbols.cu_module_unload)(module.as_ptr()) };
        if result == 0 {
            Ok(())
        } else {
            Err(CudaRuntimeError::ModuleUnloadFailed(result))
        }
    }

    pub(super) fn get_module_function(
        runtime: &CudaRuntime,
        module: CudaModuleHandle,
        name: &CStr,
    ) -> Result<CudaFunctionHandle, CudaRuntimeError> {
        set_current_context(runtime)?;

        let mut function = std::ptr::null_mut();
        // SAFETY: `module` is a loaded module owned by `CudaModule`, `name` is a valid
        // nul-terminated function name, and `function` is a valid out-pointer.
        let result = unsafe {
            (runtime.driver.symbols.cu_module_get_function)(
                &mut function,
                module.as_ptr(),
                name.as_ptr(),
            )
        };
        if result != 0 {
            return Err(CudaRuntimeError::FunctionLookupFailed(result));
        }

        NonNull::new(function).ok_or(CudaRuntimeError::FunctionLookupFailed(-1))
    }

    pub(super) fn launch_kernel(
        runtime: &CudaRuntime,
        function: CudaFunctionHandle,
        stream: CudaStreamHandle,
        config: CudaKernelLaunchConfig,
        kernel_params: &mut [*mut c_void],
    ) -> Result<(), CudaRuntimeError> {
        set_current_context(runtime)?;

        // SAFETY: `function` and `stream` belong to this runtime context. `kernel_params`
        // contains pointers to stack-owned scalar parameter values that live for the whole call;
        // CUDA copies parameter values during `cuLaunchKernel`.
        let result = unsafe {
            (runtime.driver.symbols.cu_launch_kernel)(
                function.as_ptr(),
                config.grid_dim_x(),
                config.grid_dim_y(),
                config.grid_dim_z(),
                config.block_dim_x(),
                config.block_dim_y(),
                config.block_dim_z(),
                config.shared_mem_bytes(),
                stream.as_ptr(),
                kernel_params.as_mut_ptr(),
                std::ptr::null_mut(),
            )
        };
        if result == 0 {
            Ok(())
        } else {
            Err(CudaRuntimeError::KernelLaunchFailed(result))
        }
    }

    fn set_current_context(runtime: &CudaRuntime) -> Result<(), CudaRuntimeError> {
        runtime
            .context
            .as_ref()
            .expect("CUDA runtime should own a retained context")
            .set_current()
    }

    #[cfg(windows)]
    fn driver_library_names() -> &'static [&'static str] {
        &["nvcuda.dll"]
    }

    #[cfg(target_os = "linux")]
    fn driver_library_names() -> &'static [&'static str] {
        &["libcuda.so.1", "libcuda.so"]
    }

    #[cfg(windows)]
    unsafe extern "system" {
        fn LoadLibraryA(name: *const c_char) -> *mut c_void;
        fn GetProcAddress(handle: *mut c_void, name: *const c_char) -> *mut c_void;
        fn FreeLibrary(handle: *mut c_void) -> i32;
    }

    #[cfg(target_os = "linux")]
    #[link(name = "dl")]
    unsafe extern "C" {
        fn dlopen(name: *const c_char, flags: c_int) -> *mut c_void;
        fn dlsym(handle: *mut c_void, name: *const c_char) -> *mut c_void;
        fn dlclose(handle: *mut c_void) -> c_int;
    }

    #[cfg(target_os = "linux")]
    const RTLD_NOW: c_int = 2;

    fn platform_open(name: &CStr) -> Option<NonNull<c_void>> {
        #[cfg(windows)]
        {
            // SAFETY: `name` is a valid nul-terminated library name. Null return means not found.
            NonNull::new(unsafe { LoadLibraryA(name.as_ptr()) })
        }
        #[cfg(target_os = "linux")]
        {
            // SAFETY: `name` is a valid nul-terminated library name. Null return means not found.
            NonNull::new(unsafe { dlopen(name.as_ptr(), RTLD_NOW) })
        }
    }

    fn platform_symbol(handle: NonNull<c_void>, name: &CStr) -> Option<*mut c_void> {
        #[cfg(windows)]
        {
            // SAFETY: `handle` is an owned loaded library and `name` is a nul-terminated symbol.
            NonNull::new(unsafe { GetProcAddress(handle.as_ptr(), name.as_ptr()) })
                .map(NonNull::as_ptr)
        }
        #[cfg(target_os = "linux")]
        {
            // SAFETY: `handle` is an owned loaded library and `name` is a nul-terminated symbol.
            NonNull::new(unsafe { dlsym(handle.as_ptr(), name.as_ptr()) }).map(NonNull::as_ptr)
        }
    }

    fn platform_close(handle: NonNull<c_void>) {
        #[cfg(windows)]
        {
            // SAFETY: `handle` came from `LoadLibraryA` and is closed exactly once by `Drop`.
            let _ = unsafe { FreeLibrary(handle.as_ptr()) };
        }
        #[cfg(target_os = "linux")]
        {
            // SAFETY: `handle` came from `dlopen` and is closed exactly once by `Drop`.
            let _ = unsafe { dlclose(handle.as_ptr()) };
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CudaBufferPlan {
    pub agent_count: usize,
    pub request_count: usize,
    pub dimensions: usize,
    pub k: usize,
    pub agent_id_u32_len: usize,
    pub agent_vector_f32_len: usize,
    pub request_vector_f32_len: usize,
    pub agent_score_weight_f32_len: usize,
    pub availability_u32_len: usize,
    pub output_candidate_u32_len: usize,
    pub output_effective_f32_len: usize,
    pub output_base_f32_len: usize,
    pub output_flag_u32_len: usize,
}

impl CudaBufferPlan {
    pub fn try_new(
        agent_count: usize,
        request_count: usize,
        dimensions: usize,
        k: usize,
    ) -> Result<Self, RouteError> {
        let candidate_slots = checked_mul(request_count, k)?;
        let agent_vector_f32_len = checked_mul(agent_count, dimensions)?;
        let request_vector_f32_len = checked_mul(request_count, dimensions)?;

        let plan = Self {
            agent_count,
            request_count,
            dimensions,
            k,
            agent_id_u32_len: agent_count,
            agent_vector_f32_len,
            request_vector_f32_len,
            agent_score_weight_f32_len: agent_count,
            availability_u32_len: agent_count,
            output_candidate_u32_len: candidate_slots,
            output_effective_f32_len: candidate_slots,
            output_base_f32_len: candidate_slots,
            output_flag_u32_len: request_count,
        };

        plan.checked_total_f32_len()?;
        plan.checked_total_u32_len()?;

        Ok(plan)
    }

    pub fn total_f32_len(self) -> usize {
        self.checked_total_f32_len()
            .expect("CUDA buffer plan totals are validated at construction")
    }

    pub fn total_u32_len(self) -> usize {
        self.checked_total_u32_len()
            .expect("CUDA buffer plan totals are validated at construction")
    }

    fn checked_total_f32_len(self) -> Result<usize, RouteError> {
        checked_sum(&[
            self.agent_vector_f32_len,
            self.request_vector_f32_len,
            self.agent_score_weight_f32_len,
            self.output_effective_f32_len,
            self.output_base_f32_len,
        ])
    }

    fn checked_total_u32_len(self) -> Result<usize, RouteError> {
        checked_sum(&[
            self.agent_id_u32_len,
            self.availability_u32_len,
            self.output_candidate_u32_len,
            self.output_flag_u32_len,
        ])
    }
}

fn checked_mul(left: usize, right: usize) -> Result<usize, RouteError> {
    left.checked_mul(right)
        .ok_or(RouteError::BufferSizeOverflow {
            context: CUDA_PLAN_CONTEXT,
        })
}

fn checked_sum(values: &[usize]) -> Result<usize, RouteError> {
    values.iter().try_fold(0usize, |total, value| {
        total
            .checked_add(*value)
            .ok_or(RouteError::BufferSizeOverflow {
                context: CUDA_PLAN_CONTEXT,
            })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use qtom_core::AgentLabels;

    #[test]
    fn buffer_plan_matches_flat_cuda_layout() {
        let plan = CudaBufferPlan::try_new(8, 4, 16, 3).unwrap();

        assert_eq!(plan.agent_id_u32_len, 8);
        assert_eq!(plan.agent_vector_f32_len, 128);
        assert_eq!(plan.request_vector_f32_len, 64);
        assert_eq!(plan.output_candidate_u32_len, 12);
        assert_eq!(plan.output_effective_f32_len, 12);
        assert_eq!(plan.output_base_f32_len, 12);
        assert_eq!(plan.output_flag_u32_len, 4);
    }

    #[test]
    fn buffer_plan_rejects_overflow() {
        let error = CudaBufferPlan::try_new(usize::MAX, 2, 2, 1).unwrap_err();

        assert_eq!(
            error,
            RouteError::BufferSizeOverflow {
                context: CUDA_PLAN_CONTEXT
            }
        );
    }

    #[test]
    fn device_buffer_byte_len_rejects_overflow() {
        let error = CudaDeviceBuffer::<u32>::checked_byte_len(usize::MAX).unwrap_err();

        assert_eq!(
            error,
            CudaRuntimeError::ResourceSizeOverflow(CUDA_DEVICE_BUFFER_CONTEXT)
        );
    }

    #[test]
    fn route_agents_kernel_artifact_is_embedded() {
        assert!(ROUTE_AGENTS_K1_PTX.contains(".entry qtom_route_agents_k1"));
        assert_eq!(ROUTE_AGENTS_K1_PTX.matches(".param .u64").count(), 9);
        assert_eq!(ROUTE_AGENTS_K1_PTX.matches(".param .u32").count(), 3);
        assert_eq!(ROUTE_AGENTS_K1_PTX.matches(".param .f32").count(), 0);
    }

    #[test]
    fn launch_config_uses_one_block_for_empty_work() {
        let config = CudaKernelLaunchConfig::for_1d_thread_count(0, 128).unwrap();

        assert_eq!(config.grid_dim_x(), 1);
        assert_eq!(config.block_dim_x(), 128);
    }

    #[test]
    fn launch_config_rounds_up_grid_width() {
        let config = CudaKernelLaunchConfig::for_1d_thread_count(129, 128).unwrap();

        assert_eq!(config.grid_dim_x(), 2);
        assert_eq!(config.block_dim_x(), 128);
    }

    #[test]
    fn launch_config_rejects_zero_block_width() {
        let error = CudaKernelLaunchConfig::for_1d_thread_count(1, 0).unwrap_err();

        assert_eq!(error, CudaRuntimeError::InvalidLaunchConfig("block_dim_x"));
    }

    #[test]
    fn resource_wrappers_round_trip_when_runtime_available() {
        let Ok(runtime) = CudaRuntime::initialize() else {
            return;
        };

        let stream = runtime.create_stream().unwrap();
        #[cfg(all(feature = "cuda-runtime", any(windows, target_os = "linux")))]
        {
            let start_event = runtime.create_event().unwrap();
            let stop_event = runtime.create_event().unwrap();
            start_event.record(&stream).unwrap();
            stop_event.record(&stream).unwrap();
            stop_event.synchronize().unwrap();
            assert!(
                stop_event.elapsed_since(&start_event).unwrap() < std::time::Duration::from_secs(1)
            );
            stop_event.destroy().unwrap();
            start_event.destroy().unwrap();
        }
        stream.synchronize().unwrap();
        stream.destroy().unwrap();

        let buffer = runtime.allocate_device_buffer::<f32>(16).unwrap();
        assert_eq!(buffer.len(), 16);
        assert_eq!(buffer.byte_len(), 16 * size_of::<f32>());
        assert!(!buffer.is_empty());
        buffer.free().unwrap();

        let empty = runtime.allocate_device_buffer::<u32>(0).unwrap();
        assert_eq!(empty.len(), 0);
        assert_eq!(empty.byte_len(), 0);
        assert!(empty.is_empty());
        empty.free().unwrap();
    }

    #[test]
    fn module_wrapper_loads_route_agents_function_when_runtime_available() {
        let Ok(runtime) = CudaRuntime::initialize() else {
            return;
        };

        let module = runtime.load_route_agents_module().unwrap();
        {
            let function = module.get_function(ROUTE_AGENTS_K1_KERNEL_NAME).unwrap();
            assert_eq!(function.name(), ROUTE_AGENTS_K1_KERNEL_NAME);
        }
        module.unload().unwrap();
    }

    #[cfg(feature = "cuda-runtime")]
    #[test]
    fn route_agents_kernel_launch_smoke_when_runtime_available() {
        let Ok(runtime) = CudaRuntime::initialize() else {
            return;
        };

        let plan = CudaBufferPlan::try_new(0, 0, 0, 1).unwrap();
        let agent_vectors = runtime.allocate_device_buffer::<f32>(0).unwrap();
        let agent_ids = runtime.allocate_device_buffer::<u32>(0).unwrap();
        let request_vectors = runtime.allocate_device_buffer::<f32>(0).unwrap();
        let agent_score_weights = runtime.allocate_device_buffer::<f32>(0).unwrap();
        let availability = runtime.allocate_device_buffer::<u32>(0).unwrap();
        let mut output_agent_ids = runtime.allocate_device_buffer::<u32>(0).unwrap();
        let mut output_effective_distances = runtime.allocate_device_buffer::<f32>(0).unwrap();
        let mut output_base_distances = runtime.allocate_device_buffer::<f32>(0).unwrap();
        let mut output_flags = runtime.allocate_device_buffer::<u32>(0).unwrap();

        let module = runtime.load_route_agents_module().unwrap();
        let stream = runtime.create_stream().unwrap();
        {
            let kernel = module.route_agents_kernel().unwrap();
            assert_eq!(kernel.name(), ROUTE_AGENTS_K1_KERNEL_NAME);

            let mut args = RouteAgentsKernelArgs::new(
                plan,
                &agent_vectors,
                &agent_ids,
                &request_vectors,
                &agent_score_weights,
                &availability,
                &mut output_agent_ids,
                &mut output_effective_distances,
                &mut output_base_distances,
                &mut output_flags,
            )
            .unwrap();
            assert_eq!(args.launch_config().unwrap().grid_dim_x(), 1);

            kernel.launch_and_synchronize(&stream, &mut args).unwrap();
        }
        stream.destroy().unwrap();
        module.unload().unwrap();
    }

    #[cfg(feature = "cuda-runtime")]
    #[test]
    fn route_agents_k_one_helper_decodes_results_like_cpu_for_tiny_fixture() {
        use qtom_core::CpuRouter;

        let Ok(runtime) = CudaRuntime::initialize() else {
            return;
        };

        let coefficients = ScoreCoefficients::default();
        let agents = vec![
            agent(1, &[0.0, 0.0]),
            agent(2, &[0.1, 0.0]),
            agent(3, &[1.0, 1.0]),
        ];
        let states = vec![
            AgentRuntimeState::unavailable(),
            AgentRuntimeState {
                queue_depth_norm: 0.1,
                latency_norm: 0.2,
                cache_pressure_norm: 0.3,
                availability: 1,
            },
            AgentRuntimeState::available(),
        ];
        let requests = vec![
            RoutingRequest {
                task_id: 10,
                vector: vec![0.0, 0.0],
                k: 1,
                fallback_generalist_id: 999,
                radius_max_threshold: 10.0,
            },
            RoutingRequest {
                task_id: 11,
                vector: vec![0.95, 1.0],
                k: 1,
                fallback_generalist_id: 999,
                radius_max_threshold: 10.0,
            },
            RoutingRequest {
                task_id: 12,
                vector: vec![10.0, 10.0],
                k: 1,
                fallback_generalist_id: 999,
                radius_max_threshold: 0.1,
            },
        ];

        let cpu = CpuRouter::new(agents.clone(), coefficients).with_debug_observed(false);
        let expected = cpu.route_batch_with_workers(&requests, &states, 1).unwrap();
        let route_table = AgentRouteTable::from_agents(agents).unwrap();
        let actual =
            execute_route_agents_k1(&runtime, &route_table, coefficients, &requests, &states)
                .unwrap();

        assert_results_close(&actual, &expected);
    }

    #[cfg(feature = "cuda-runtime")]
    #[test]
    fn route_agents_k_one_helper_matches_cpu_for_deterministic_fixture() {
        use qtom_core::{CpuRouter, FixtureConfig, generate_fixture};

        let Ok(runtime) = CudaRuntime::initialize() else {
            return;
        };

        let config = FixtureConfig {
            agent_count: 128,
            task_count: 32,
            dimensions: 16,
            k: 1,
            seed: 0x5154_4f4d,
        };
        let fixture = generate_fixture(config);
        let coefficients = ScoreCoefficients::default();
        let cpu = CpuRouter::new(fixture.agents.clone(), coefficients).with_debug_observed(false);
        let expected = cpu
            .route_batch_with_workers(&fixture.requests, &fixture.states, 1)
            .unwrap();
        let route_table = AgentRouteTable::from_agent_slice(&fixture.agents).unwrap();

        let actual = execute_route_agents_k1(
            &runtime,
            &route_table,
            coefficients,
            &fixture.requests,
            &fixture.states,
        )
        .unwrap();

        assert_results_close(&actual, &expected);
    }

    #[cfg(feature = "cuda-runtime")]
    #[test]
    fn router_route_batch_matches_cpu_for_deterministic_k_one_fixture() {
        use qtom_core::{CpuRouter, FixtureConfig, generate_fixture};

        let config = FixtureConfig {
            agent_count: 128,
            task_count: 32,
            dimensions: 16,
            k: 1,
            seed: 0x5154_4f4d,
        };
        let fixture = generate_fixture(config);
        let coefficients = ScoreCoefficients::default();
        let cpu = CpuRouter::new(fixture.agents.clone(), coefficients).with_debug_observed(false);
        let expected = cpu
            .route_batch_with_workers(&fixture.requests, &fixture.states, 1)
            .unwrap();
        let router = CudaRouter::new(fixture.agents, coefficients);

        let actual = match router.route_batch(&fixture.requests, &fixture.states) {
            Ok(results) => results,
            Err(RouteError::BackendUnavailable { .. }) => return,
            Err(error) => panic!("unexpected CUDA route error: {error}"),
        };

        assert_results_close(&actual, &expected);
    }

    #[cfg(feature = "cuda-runtime")]
    #[test]
    fn router_route_batch_keeps_k_greater_than_one_closed() {
        let router = CudaRouter::new(vec![agent(1, &[0.0, 0.0])], ScoreCoefficients::default());

        let error = router
            .route_batch(
                &[request(1, &[0.0, 0.0], 2, 999, 1.0)],
                &[AgentRuntimeState::available()],
            )
            .unwrap_err();

        assert_eq!(
            error,
            RouteError::BackendUnavailable {
                backend: CUDA_BACKEND_NAME,
                reason: CUDA_K_ONE_ONLY_REASON
            }
        );
    }

    #[cfg(feature = "cuda-runtime")]
    #[test]
    fn k_one_helper_rejects_unsupported_k_before_launch() {
        let route_table = AgentRouteTable::from_agents(vec![agent(1, &[0.0, 0.0])]).unwrap();
        let requests = vec![RoutingRequest {
            task_id: 1,
            vector: vec![0.0, 0.0],
            k: 2,
            fallback_generalist_id: 999,
            radius_max_threshold: 1.0,
        }];
        let states = vec![AgentRuntimeState::available()];

        let error = validate_k_one_inputs(&route_table, &requests, &states).unwrap_err();

        assert_eq!(error, CudaRouteExecutionError::UnsupportedK { actual: 2 });
    }

    #[cfg(feature = "cuda-runtime")]
    #[test]
    fn k_one_helper_rejects_bad_request_dimensions_before_launch() {
        let route_table = AgentRouteTable::from_agents(vec![agent(1, &[0.0, 0.0])]).unwrap();
        let requests = vec![RoutingRequest {
            task_id: 1,
            vector: vec![0.0],
            k: 1,
            fallback_generalist_id: 999,
            radius_max_threshold: 1.0,
        }];
        let states = vec![AgentRuntimeState::available()];

        let error = validate_k_one_inputs(&route_table, &requests, &states).unwrap_err();

        assert_eq!(
            error,
            CudaRouteExecutionError::Route(RouteError::DimensionMismatch {
                expected: 2,
                actual: 1,
                context: "routing request"
            })
        );
    }

    #[cfg(feature = "cuda-runtime")]
    #[test]
    fn k_one_helper_rejects_state_length_mismatch_before_launch() {
        let route_table =
            AgentRouteTable::from_agents(vec![agent(1, &[0.0, 0.0]), agent(2, &[1.0, 1.0])])
                .unwrap();
        let requests = vec![RoutingRequest {
            task_id: 1,
            vector: vec![0.0, 0.0],
            k: 1,
            fallback_generalist_id: 999,
            radius_max_threshold: 1.0,
        }];
        let states = vec![AgentRuntimeState::available()];

        let error = validate_k_one_inputs(&route_table, &requests, &states).unwrap_err();

        assert_eq!(
            error,
            CudaRouteExecutionError::Route(RouteError::StateLengthMismatch {
                agents: 2,
                states: 1
            })
        );
    }

    #[test]
    fn runtime_status_is_consistent() {
        let status = detect_cuda_runtime();

        if status.available {
            assert!(status.device_count > 0);
            assert_eq!(status.reason, CUDA_RUNTIME_AVAILABLE_REASON);
            assert_eq!(status.error_code, None);
        } else {
            assert_eq!(status.device_count, 0);
            assert!(!status.reason.is_empty());
        }
    }

    #[cfg(not(feature = "cuda-runtime"))]
    #[test]
    fn runtime_status_reports_disabled_without_feature() {
        let status = detect_cuda_runtime();

        assert_eq!(
            status,
            CudaRuntimeStatus::unavailable(CUDA_RUNTIME_FEATURE_DISABLED_REASON, None)
        );
    }

    #[test]
    fn backend_status_reports_runtime_but_keeps_router_unavailable() {
        let router = CudaRouter::new(vec![agent(1, &[0.0, 0.0])], ScoreCoefficients::default());
        let status = router.status();

        assert!(!status.available);
        assert_eq!(status.reason, SCAFFOLD_REASON);
        assert_eq!(status.runtime, router.runtime_status());
    }

    #[test]
    fn router_empty_batch_returns_empty_results() {
        let router = CudaRouter::new(vec![agent(1, &[0.0, 0.0])], ScoreCoefficients::default());

        let results = router
            .route_batch(&[], &[AgentRuntimeState::available()])
            .unwrap();

        assert!(results.is_empty());
    }

    #[cfg(not(feature = "cuda-runtime"))]
    #[test]
    fn router_reports_unavailable_without_cuda_runtime_feature() {
        let router = CudaRouter::new(vec![agent(1, &[0.0, 0.0])], ScoreCoefficients::default());

        let error = router
            .route_batch(
                &[request(1, &[0.0, 0.0], 1, 999, 1.0)],
                &[AgentRuntimeState::available()],
            )
            .unwrap_err();

        assert_eq!(
            error,
            RouteError::BackendUnavailable {
                backend: CUDA_BACKEND_NAME,
                reason: CUDA_RUNTIME_FEATURE_DISABLED_REASON
            }
        );
    }

    #[test]
    fn router_rejects_invalid_agent_layout_before_unavailable_status() {
        let router = CudaRouter::new(
            vec![agent(1, &[0.0]), agent(2, &[0.0, 1.0])],
            ScoreCoefficients::default(),
        );

        let error = router
            .buffer_plan(1, 1)
            .expect_err("invalid route table should be preserved");

        assert_eq!(
            error,
            RouteError::DimensionMismatch {
                expected: 1,
                actual: 2,
                context: "agent vector"
            }
        );
    }

    fn agent(id: u32, vector: &[f32]) -> AgentProfile {
        AgentProfile {
            id,
            vector: vector.to_vec(),
            labels: AgentLabels::default(),
        }
    }

    fn request(
        task_id: u64,
        vector: &[f32],
        k: usize,
        fallback_generalist_id: u32,
        radius_max_threshold: f32,
    ) -> RoutingRequest {
        RoutingRequest {
            task_id,
            vector: vector.to_vec(),
            k,
            fallback_generalist_id,
            radius_max_threshold,
        }
    }

    #[cfg(feature = "cuda-runtime")]
    fn assert_results_close(actual: &[RoutingResult], expected: &[RoutingResult]) {
        assert_eq!(actual.len(), expected.len());
        for (actual, expected) in actual.iter().zip(expected) {
            assert_eq!(actual.task_id, expected.task_id);
            assert_eq!(actual.used_fallback, expected.used_fallback);
            assert_eq!(
                actual.ideal_candidate_unavailable,
                expected.ideal_candidate_unavailable
            );
            assert_eq!(actual.debug, expected.debug);
            assert_eq!(
                actual.available_candidates.len(),
                expected.available_candidates.len()
            );
            for (actual_candidate, expected_candidate) in actual
                .available_candidates
                .iter()
                .zip(expected.available_candidates.iter())
            {
                assert_eq!(actual_candidate.agent_id, expected_candidate.agent_id);
                assert_eq!(actual_candidate.available, expected_candidate.available);
                assert_close(
                    actual_candidate.effective_distance,
                    expected_candidate.effective_distance,
                );
                assert_close(
                    actual_candidate.base_distance,
                    expected_candidate.base_distance,
                );
                assert_close(actual_candidate.omega, expected_candidate.omega);
                assert_close(
                    actual_candidate.queue_penalty,
                    expected_candidate.queue_penalty,
                );
                assert_close(
                    actual_candidate.latency_penalty,
                    expected_candidate.latency_penalty,
                );
                assert_close(
                    actual_candidate.cache_penalty,
                    expected_candidate.cache_penalty,
                );
            }
        }
    }

    #[cfg(feature = "cuda-runtime")]
    fn assert_close(actual: f32, expected: f32) {
        if actual.is_infinite() || expected.is_infinite() {
            assert_eq!(actual, expected);
            return;
        }
        assert!(
            (actual - expected).abs() <= 1.0e-6,
            "expected {expected}, got {actual}"
        );
    }
}
