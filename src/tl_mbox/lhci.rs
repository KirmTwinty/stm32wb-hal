use crate::pac;
use crate::tl_mbox::cmd::CmdPacket;
use crate::tl_mbox::consts::TlPacketType;
use crate::tl_mbox::evt::{CcEvt, EvtPacket, EvtSerial};
use crate::tl_mbox::{
    DeviceInfoTable, RssInfoTable, SafeBootInfoTable, WirelessFwInfoTable, TL_EVT_HEADER_SIZE,
    TL_REF_TABLE,
};

use stm32_device_signature;

const TL_BLEEVT_CC_OPCODE: u8 = 0x0e;
#[allow(dead_code)] // Not used currently but reserved
const TL_BLEEVT_CS_OPCODE: u8 = 0x0f;

const LHCI_OPCODE_C1_DEVICE_INF: u16 = 0xfd62;

const PACKAGE_DATA_PTR: *const u8 = 0x1FFF_7500 as _;
const UID64_PTR: *const u32 = 0x1FFF_7580 as _;

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct LhciC1DeviceInformationCcrp {
    pub status: u8,
    pub rev_id: u16,
    pub dev_code_id: u16,
    pub package_type: u8,
    pub device_type_id: u8,
    pub st_company_id: u32,
    pub uid64: u32,

    pub uid96_0: u32,
    pub uid96_1: u32,
    pub uid96_2: u32,

    pub safe_boot_info_table: SafeBootInfoTable,
    pub rss_info_table: RssInfoTable,
    pub wireless_fw_info_table: WirelessFwInfoTable,

    pub app_fw_inf: u32,
}

impl LhciC1DeviceInformationCcrp {
    pub fn new() -> Self {
        let DeviceInfoTable {
            safe_boot_info_table,
            rss_info_table,
            wireless_fw_info_table,
        } = unsafe { &*(&*TL_REF_TABLE.as_ptr()).device_info_table }.clone();

        let dbgmcu = unsafe { pac::Peripherals::steal() }.DBGMCU;
        let rev_id = dbgmcu.idcode.read().rev_id().bits();
        let dev_code_id = dbgmcu.idcode.read().dev_id().bits();

        let device_id = stm32_device_signature::device_id();
        let uid96_0 = (device_id[3] as u32) << 24
            | (device_id[2] as u32) << 16
            | (device_id[1] as u32) << 8
            | device_id[0] as u32;
        let uid96_1 = (device_id[7] as u32) << 24
            | (device_id[6] as u32) << 16
            | (device_id[5] as u32) << 8
            | device_id[4] as u32;
        let uid96_2 = (device_id[11] as u32) << 24
            | (device_id[10] as u32) << 16
            | (device_id[9] as u32) << 8
            | device_id[8] as u32;

        let package_type = unsafe { *PACKAGE_DATA_PTR };
        let uid64 = unsafe { *UID64_PTR };
        let st_company_id = unsafe { *UID64_PTR.offset(1) } >> 8 & 0x00FF_FFFF;
        let device_type_id = (unsafe { *UID64_PTR.offset(1) } & 0x000000FF) as u8;

        LhciC1DeviceInformationCcrp {
            status: 0,
            rev_id,
            dev_code_id,
            package_type,
            device_type_id,
            st_company_id,
            uid64,
            uid96_0,
            uid96_1,
            uid96_2,
            safe_boot_info_table,
            rss_info_table,
            wireless_fw_info_table,
            app_fw_inf: (1 << 8), // 0.0.1
        }
    }

    pub fn write(&self, cmd_packet: &mut CmdPacket) {
        let self_size = core::mem::size_of::<LhciC1DeviceInformationCcrp>();

        unsafe {
            let cmd_packet_ptr: *mut CmdPacket = cmd_packet;
            let evt_packet_ptr: *mut EvtPacket = cmd_packet_ptr.cast();

            let evt_serial: *mut EvtSerial = &mut (*evt_packet_ptr).evt_serial;
            let evt_payload: *mut u8 = (&mut *evt_serial).evt.payload.as_mut_ptr();
            let evt_cc: *mut CcEvt = evt_payload.cast();
            let evt_cc_payload_buf: *mut u8 = (&mut *evt_cc).payload.as_mut_ptr();

            (*evt_serial).kind = TlPacketType::LocRsp as u8;
            (*evt_serial).evt.evt_code = TL_BLEEVT_CC_OPCODE;
            (*evt_serial).evt.payload_len = TL_EVT_HEADER_SIZE as u8 + self_size as u8;

            (*evt_cc).cmd_code = LHCI_OPCODE_C1_DEVICE_INF;
            (*evt_cc).num_cmd = 1;

            let self_ptr: *const LhciC1DeviceInformationCcrp = self;
            let self_buf: *const u8 = self_ptr.cast();

            core::ptr::copy(self_buf, evt_cc_payload_buf, self_size);
        }
    }
}
