use crate::display::mccs::MonitorCapabilities;
use std::fmt;
use winapi::shared::minwindef::BYTE;
use winapi::shared::minwindef::DWORD;
use winapi::shared::minwindef::{LPARAM, LPDWORD};
use winapi::shared::windef::{HDC, HMONITOR, LPRECT};
use winapi::um::lowlevelmonitorconfigurationapi::SetVCPFeature;
use winapi::um::lowlevelmonitorconfigurationapi::{
    CapabilitiesRequestAndCapabilitiesReply, GetCapabilitiesStringLength,
};
use winapi::um::physicalmonitorenumerationapi::{
    GetNumberOfPhysicalMonitorsFromHMONITOR, GetPhysicalMonitorsFromHMONITOR, PHYSICAL_MONITOR,
};
use winapi::um::winuser::EnumDisplayMonitors;

#[derive(Debug, Clone)]
pub struct MonitorError(&'static str);

impl std::fmt::Display for MonitorError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Monitor Error: {}", self.0)
    }
}

impl std::error::Error for MonitorError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

#[derive(Debug, FromPrimitive, Copy, Clone)]
pub enum MonitorInput {
    AnalogVideo1 = 1,
    AnalogVideo2,
    DVI1,
    DVI2,
    CompositeVideo1,
    CompositeVideo2,
    SVideo1,
    SVideo2,
    Tuner1,
    Tuner2,
    Tuner3,
    ComponentVideo1,
    ComponentVideo2,
    ComponentVideo3,
    DisplayPort1,
    DisplayPort2,
    HDMI1,
    HDMI2,
    Unknown,
    Reserved,
}

impl fmt::Display for MonitorInput {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use MonitorInput::*;

        let s = match self {
            AnalogVideo1 => "Analog Video 1",
            AnalogVideo2 => "Analog Video 2",
            DVI1 => "DVI 1",
            DVI2 => "DVI 2",
            CompositeVideo1 => "Composite Video 1",
            CompositeVideo2 => "Composite Video 2",
            SVideo1 => "SVideo 1",
            SVideo2 => "SVideo 2",
            Tuner1 => "Tuner 1",
            Tuner2 => "Tuner 2",
            Tuner3 => "Tuner 3",
            ComponentVideo1 => "Component Video 1",
            ComponentVideo2 => "Component Video 2",
            ComponentVideo3 => "Component Video 3",
            DisplayPort1 => "DP 1",
            DisplayPort2 => "DP 2",
            HDMI1 => "HDMI 1",
            HDMI2 => "HDMI 2",
            _ => "Unknown",
        };

        write!(f, "{}", s)
    }
}

#[derive(Default, Clone)]
pub struct Monitor {
    pub cap_string: Option<String>,
    pub capabilities: Option<MonitorCapabilities>,
    pub phys_mons: PHYSICAL_MONITOR,
    pub inputs: Vec<MonitorInput>,
}

impl fmt::Display for Monitor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "<{}>", "Monitor")
    }
}

impl Monitor {
    fn set_vcp_feature(&self, code: BYTE, new_value: DWORD) -> Result<(), MonitorError> {
        let hmonitor = self.phys_mons.hPhysicalMonitor;

        unsafe {
            let result = SetVCPFeature(hmonitor, code, new_value);

            return match result {
                1 => Ok(()),
                _ => Err(MonitorError("Failed to set value for monitor")),
            };
        }
    }

    pub fn set_input(&self, input: MonitorInput) -> Result<(), MonitorError> {
        let code = 0x60; // Input Select VCP Code
        let input_code = input as u32;

        self.set_vcp_feature(code, input_code).unwrap();

        Ok(())
    }
}

pub struct MonitorManager {
    pub monitors: Vec<Monitor>,
}

impl MonitorManager {
    pub fn new() -> Result<MonitorManager, MonitorError> {
        let mut manager = MonitorManager { monitors: vec![] };

        let all_monitors = manager.get_all_monitors();

        match all_monitors {
            Ok(monitors) => manager.monitors = monitors,
            Err(e) => panic!(e),
        }

        Ok(manager)
    }

    fn enum_display_monitors(&self) -> Vec<HMONITOR> {
        let hdc = std::ptr::null_mut();
        let lprc_clip = std::ptr::null_mut();

        let mut monitors: Box<Vec<HMONITOR>> = Box::new(vec![]);

        unsafe {
            let mons_ptr = Box::into_raw(monitors);
            let mons_lparam: LPARAM = std::mem::transmute(mons_ptr);

            EnumDisplayMonitors(hdc, lprc_clip, Some(lpfn_enum_callback), mons_lparam);

            monitors = Box::from_raw(mons_ptr);

            return monitors.to_vec();
        }
    }

    fn get_number_of_physical_monitors_from_hmonitor(&self, hmonitor: HMONITOR) -> i32 {
        let mut num_phys_monitors: Box<i32> = Box::new(0);

        unsafe {
            let num_ptr = Box::into_raw(num_phys_monitors);
            let num_lpdword: LPDWORD = std::mem::transmute(num_ptr);

            GetNumberOfPhysicalMonitorsFromHMONITOR(hmonitor, num_lpdword);

            num_phys_monitors = Box::from_raw(num_ptr);

            return *num_phys_monitors;
        }
    }

    fn get_physical_monitors_from_hmonitor(
        &self,
        monitor: HMONITOR,
        phys_mons_size: i32,
    ) -> Vec<PHYSICAL_MONITOR> {
        let mut phys_mons: Vec<PHYSICAL_MONITOR> =
            vec![Default::default(); phys_mons_size as usize];

        unsafe {
            GetPhysicalMonitorsFromHMONITOR(
                monitor,
                phys_mons.len() as u32,
                phys_mons.as_mut_ptr(),
            );

            return phys_mons;
        }
    }

    fn get_capabilities_string_length(&self, phys_mon: PHYSICAL_MONITOR) -> i32 {
        let mut cap_string_len: Box<i32> = Box::new(0);

        unsafe {
            let cap_len_ptr = Box::into_raw(cap_string_len);
            let cap_lpdword: LPDWORD = std::mem::transmute(cap_len_ptr);

            GetCapabilitiesStringLength(phys_mon.hPhysicalMonitor, cap_lpdword);

            cap_string_len = Box::from_raw(cap_len_ptr);

            return *cap_string_len;
        }
    }

    fn capabilities_request_and_capabilities_reply(
        &self,
        phys_mon: PHYSICAL_MONITOR,
        cap_string_len: i32,
    ) -> String {
        unsafe {
            let mut cap_string_buf: Vec<i8> = vec![0; cap_string_len as usize];

            CapabilitiesRequestAndCapabilitiesReply(
                phys_mon.hPhysicalMonitor,
                cap_string_buf.as_mut_ptr(),
                cap_string_len as u32,
            );

            let cap_string =
                String::from_utf8(cap_string_buf.iter().map(|&c| c as u8).collect()).unwrap();

            return String::from(cap_string.trim_matches(char::from(0)));
        }
    }

    pub fn get_all_inputs_for_monitor(
        &self,
        capabilities: &MonitorCapabilities,
    ) -> Result<Vec<MonitorInput>, MonitorError> {
        let input_select_values = capabilities
            .vcp_codes
            .iter()
            .find(|cmd| &cmd.command == "60")
            .unwrap()
            .values
            .iter()
            .map(|v| match &v.command[..] {
                "01" => MonitorInput::AnalogVideo1,
                "02" => MonitorInput::AnalogVideo2,
                "03" => MonitorInput::DVI1,
                "04" => MonitorInput::DVI2,
                "05" => MonitorInput::CompositeVideo1,
                "06" => MonitorInput::CompositeVideo2,
                "07" => MonitorInput::SVideo1,
                "08" => MonitorInput::SVideo2,
                "09" => MonitorInput::Tuner1,
                "0A" => MonitorInput::Tuner2,
                "0B" => MonitorInput::Tuner3,
                "0C" => MonitorInput::ComponentVideo1,
                "0D" => MonitorInput::ComponentVideo2,
                "0E" => MonitorInput::ComponentVideo3,
                "0F" => MonitorInput::DisplayPort1,
                "10" => MonitorInput::DisplayPort2,
                "11" => MonitorInput::HDMI1,
                "12" => MonitorInput::HDMI2,
                _ => MonitorInput::Unknown,
            })
            .collect();

        Ok(input_select_values)
    }

    pub fn get_all_monitors(&self) -> Result<Vec<Monitor>, MonitorError> {
        let display_mons = self.enum_display_monitors();

        let mut monitors: Vec<Monitor> = vec![];

        for mon_ref in display_mons {
            let phys_num = self.get_number_of_physical_monitors_from_hmonitor(mon_ref);
            let phys_mons = self.get_physical_monitors_from_hmonitor(mon_ref, phys_num);

            for phys_mon in phys_mons {
                let mut mon = Monitor {
                    ..Default::default()
                };

                let cap_str_len = self.get_capabilities_string_length(phys_mon);

                let cap_reply_str =
                    self.capabilities_request_and_capabilities_reply(phys_mon, cap_str_len);

                if cap_reply_str.is_empty() || cap_reply_str.eq("") {
                    continue;
                }

                mon.cap_string = Some(cap_reply_str.clone());

                let caps = MonitorCapabilities::from_cap_string(cap_reply_str);
                match caps {
                    Ok(result) => {
                        mon.phys_mons = phys_mon;

                        let inputs = self.get_all_inputs_for_monitor(&result).unwrap();

                        mon.capabilities = Some(result);
                        mon.inputs = inputs;

                        monitors.push(mon);
                    }
                    Err(_) => {}
                }
            }
        }

        Ok(monitors)
    }
}

unsafe extern "system" fn lpfn_enum_callback(
    hmon: HMONITOR,
    _hdc: HDC,
    _lprect: LPRECT,
    lparam: LPARAM,
) -> i32 {
    let mons_ptr: *mut Vec<HMONITOR> = std::mem::transmute(lparam);
    let mons_ref = &mut *mons_ptr;

    mons_ref.push(hmon);

    return 1;
}
