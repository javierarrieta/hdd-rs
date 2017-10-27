pub enum Command {
	Identify = 0xec,
	SMART = 0xb0,
}
pub enum SMARTFeature {
	ReadValues = 0xd0, // in ATA8-ACS it's called 'SMART READ DATA', which is a bit unclear to people not familiar with ATA… or sometimes even to some who knows ATA well
	ReadThresholds = 0xd1,
	ReturnStatus = 0xda,
}

// data port is omitted for obvious reasons
pub struct RegistersRead {
	pub error: u8,

	pub sector_count: u8,

	pub sector: u8, // lba (least significant bits)
	pub cyl_low: u8, // lba
	pub cyl_high: u8, // lba
	pub device: u8, // lba (most significant bits); aka drive/head, device/head, select

	pub status: u8,
}
pub struct RegistersWrite {
	pub features: u8,

	pub sector_count: u8,

	pub sector: u8,
	pub cyl_low: u8,
	pub cyl_high: u8,
	pub device: u8,

	pub command: u8,
}

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use self::linux::*;

#[cfg(target_os = "freebsd")]
mod freebsd;
#[cfg(target_os = "freebsd")]
pub use self::freebsd::*;