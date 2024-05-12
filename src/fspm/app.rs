use crate::{scheduler::TaskCallback, PNet};

pub enum EventValues {
    Abort,
    Startup,
    Prmend,
    AppReady,
    EventData,
}

pub struct PnioStatus {
    pub error_code: u8,
    pub error_decode: u8,
    pub error_code_1: u8,
    pub error_code_2: u8,
}

pub struct EventResult {
    pub pnio_status: PnioStatus,
}

pub enum ControlCommand {
    PrmBegin,
    PrmEnd,
    AppReady,
    Release,
    ReadyForCompanion,
    ReadyForRtc3,
}

pub struct AlarmArgument {
    pub api_id: usize,
    pub slot_number: usize,
    pub subslot_number: usize,
    pub alarm_type: usize,
    pub sequence_number: usize,
    pub alarm_specifier: AlarmSpecifier,
}

pub struct AlarmSpecifier {
    pub channel_diagnosis: bool,
    pub manufacturer_diagnosis: bool,
    pub submodule_diagnosis: bool,
    pub ar_diagnosis: bool,
}

pub trait App {
    fn connect_ind_callback<T: App + Copy, U: TaskCallback + Copy>(
        &mut self,
        pnet: &mut PNet<T, U>,
        arep: usize,
        result: EventResult,
    );
    fn release_ind_callback<T: App + Copy, U: TaskCallback + Copy>(
        &mut self,
        pnet: &mut PNet<T, U>,
        arep: usize,
        result: EventResult,
    );
    fn dcontrol_ind_callback<T: App + Copy, U: TaskCallback + Copy>(
        &mut self,
        pnet: &mut PNet<T, U>,
        arep: usize,
        control_command: ControlCommand,
        result: EventResult,
    );
    fn sm_released_ind_callback<T: App + Copy, U: TaskCallback + Copy>(
        &mut self,
        pnet: &mut PNet<T, U>,
        arep: usize,
        api: usize,
        slot_number: usize,
        subslot_number: usize,
        result: EventResult,
    );
    fn ccontrol_cnf_callback<T: App + Copy, U: TaskCallback + Copy>(
        &mut self,
        pnet: &mut PNet<T, U>,
        arep: usize,
        result: EventResult,
    );
    fn state_ind_callback<T: App + Copy, U: TaskCallback + Copy>(
        &mut self,
        pnet: &mut PNet<T, U>,
        arep: usize,
        state: EventValues,
    );
    fn read_ind_callback<T: App + Copy, U: TaskCallback + Copy>(
        &mut self,
        pnet: &mut PNet<T, U>,
        arep: usize,
        api: usize,
        slot: usize,
        subslot: usize,
        idx: usize,
        sequence_number: usize,
        read_data: usize,
        read_length: usize,
        result: EventResult,
    );
    fn write_ind_callback<T: App + Copy, U: TaskCallback + Copy>(
        &mut self,
        pnet: &mut PNet<T, U>,
        arep: usize,
        api: usize,
        slot: usize,
        subslot: usize,
        idx: usize,
        sequence_number: usize,
        write_length: usize,
        write_data: usize,
        result: EventResult,
    );
    fn expect_module_ind_callback<T: App + Copy, U: TaskCallback + Copy>(
        &mut self,
        pnet: &mut PNet<T, U>,
        api: usize,
        slot: usize,
        module_ident: usize,
    );
    fn new_data_status_ind_callback<T: App + Copy, U: TaskCallback + Copy>(
        &mut self,
        pnet: &mut PNet<T, U>,
        arep: usize,
        crep: usize,
        changes: usize,
        data_status: usize,
    );
    fn alarm_ind_callback<T: App + Copy, U: TaskCallback + Copy>(
        &mut self,
        pnet: &mut PNet<T, U>,
        arep: usize,
        alarm_argument: AlarmArgument,
        data_len: usize,
        data_usi: usize,
        data: usize,
    );
    fn alarm_cnf_callback<T: App + Copy, U: TaskCallback + Copy>(
        &mut self,
        pnet: &mut PNet<T, U>,
        arep: usize,
        status: PnioStatus,
    );
    fn alarm_ack_cnf_callback<T: App + Copy, U: TaskCallback + Copy>(
        &mut self,
        pnet: &mut PNet<T, U>,
        arep: usize,
        res: usize,
    );
    fn reset_ind_callback<T: App + Copy, U: TaskCallback + Copy>(
        &mut self,
        pnet: &mut PNet<T, U>,
        should_reset_app: bool,
        reset_mode: usize,
    );
    fn signal_led_ind<T: App + Copy, U: TaskCallback + Copy>(
        &mut self,
        pnet: &mut PNet<T, U>,
        led_state: bool,
    );
}
