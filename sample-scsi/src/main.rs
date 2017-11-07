extern crate hdd;
use hdd::Device;
use hdd::scsi::SCSIDevice;
use hdd::scsi::pages::Pages;
use hdd::scsi::data::{inquiry, log_page};
use hdd::scsi::data::vpd::device_id;

#[macro_use]
extern crate clap;
use clap::{App, Arg};

extern crate separator;
use separator::Separatable;
extern crate number_prefix;
use number_prefix::{decimal_prefix, binary_prefix, Standalone, Prefixed};

fn print_hex(data: &[u8]) {
	for i in 0..data.len() {
		if i % 16 == 0 { print!("\n"); }
		print!(" {:02x}", data[i]);
	}
	print!("\n");
}

fn query(what: &str, dev: &Device, vpd: bool, page: u8, verbose: bool) -> Vec<u8> {
	print!("=== {} ===\n", what);
	let (sense, data) = dev.scsi_inquiry(vpd, page).unwrap();

	if verbose {
		print!("sense:");
		print_hex(&sense);

		print!("data: len={}", data.len());
		print_hex(&data);
	}

	data
}

fn ask_log(what: &str, dev: &Device, page: u8, subpage: u8, verbose: bool) -> Vec<u8> {
	print!("=== {} ===\n", what);
	let (sense, data) = dev.log_sense(
		false, // changed
		false, // save_params
		false, // default
		false, // threshold
		page, subpage,
		0, // param_ptr
	).unwrap();

	if verbose {
		print!("sense:");
		print_hex(&sense);

		print!("data: len={}", data.len());
		print_hex(&data);
	}

	data
}

fn main() {
	let args = App::new("sample-scsi")
		.version(crate_version!())
		.arg(Arg::with_name("device")
			.help("Device to query")
			.required(true)
			.index(1)
		)
		.arg(Arg::with_name("verbose")
			.short("v")
			.long("verbose")
			.help("show hex data")
		)
		.get_matches();

	let dev = Device::open(
		args.value_of("device").unwrap()
	).unwrap();
	let verbose = args.is_present("verbose");

	let (_, lba, block_size) = dev.read_capacity_10(None).unwrap();
	let cap = lba as u64 * block_size as u64;
	print!("Capacity: {} × {}\n", lba, block_size);
	print!("          {} bytes\n", cap.separated_string());
	print!("          ({}, {})\n",
		match decimal_prefix(cap as f32) {
			Prefixed(p, x) => format!("{:.1} {}B", x, p),
			Standalone(x)  => format!("{} bytes", x),
		},
		match binary_prefix(cap as f32) {
			Prefixed(p, x) => format!("{:.1} {}B", x, p),
			Standalone(x)  => format!("{} bytes", x),
		},
	);

	let data = query("Inquiry", &dev, false, 0, verbose);
	print!("{:#?}\n", inquiry::parse_inquiry(&data));

	let data = query("[00] Supported VPD pages", &dev, true, 0, verbose);
	let len = data[3];
	print!("supported:");
	for i in 0..len {
		print!(" {:02x}", data[(i+4) as usize]);
	}
	print!("\n");

	let data = query("[83] Device Information", &dev, true, 0x83, verbose);
	let len = ((data[2] as usize) << 8) + (data[3] as usize);

	print!("descriptors:\n");
	for d in device_id::parse(&data[4 .. 4+len]) {
		print!("{:?}\n", d);

		// TODO? from_utf8 it right in hdd::data::vpd::device_id
		if d.codeset == device_id::CodeSet::ASCII {
			match d.id {
				device_id::Identifier::VendorSpecific(i) |
				device_id::Identifier::FCNameIdentifier(i) => {
					print!(">>> {:?}\n", std::str::from_utf8(i));
				},
				device_id::Identifier::Generic { vendor_id: v, id: i } => {
					print!(">>> {:?}\n", std::str::from_utf8(v));
					print!(">>> {:?}\n", std::str::from_utf8(i));
				},
				_ => (),
			}
		}
	}

	dev.supported_pages().map(|pages| {
		for p in pages {
			if p == 00 { continue; }

			let name = match p {
				0x02 => "Write Error Counter",
				0x03 => "Read Error Counter",
				0x04 => "Read Reverse Error Counter",
				0x05 => "Verify Error Counter",
				0x06 => "Non-Medium Error",
				0x0d => "Temperature",
				0x0e => "Start-Stop Cycle Counter",
				0x10 => "Self-Test results",
				0x2f => "Informational Exceptions",
				0x30...0x3e => "(Vendor-Specific)",
				0x3f => "(Reserved)",
				_ => "?",
			};

			let data = ask_log(&format!("[{:02x}] {}", p, name), &dev, p, 0x00, verbose);
			let page = log_page::parse(&data);
			if let Some(page) = page {
				match p {
					0x02 => { print!("Write Error Counters: {:#?}\n", dev.write_error_counters()) }
					0x03 => { print!("Read Error Counters: {:#?}\n", dev.read_error_counters()) }
					0x04 => { print!("Read Reverse Error Counters: {:#?}\n", dev.read_reverse_error_counters()) }
					0x05 => { print!("Verify Error Counters: {:#?}\n", dev.verify_error_counters()) }
					0x06 => { print!("Non-Medium Error Count: {:?}\n", dev.non_medium_error_count()) }
					0x0d => {
						dev.temperature().map(|(temp, ref_temp)| {
							print!("Temperature: {:?} °C\n", temp);
							print!("Reference temperature: {:?} °C\n", ref_temp);
						});
					},
					0x0e => { print!("{:#?}\n", dev.dates_and_cycle_counters()) }
					0x10 => { print!("{:#?}\n", dev.self_test_results()) }
					0x2f => { print!("{:#?}\n", dev.informational_exceptions()) }
					_ => {
						print!("{:?}\n", page);
						print!("{:#?}\n", page.parse_params());
					},
				}
			}
		}
	});

	/*
	// TODO tell whether subpages are supported at all
	let data = ask_log("[00/ff] Supported Log Pages/Subpages", &dev, 0x00, 0xff, verbose);
	let page = log_page::parse(&data);
	if let Some(page) = page {
		for psp in page.data[..].chunks(2) {
			let (page, subpage) = (psp[0], psp[1]);

			let data = ask_log(&format!("[{:02x}/{:02x}] ?", page, subpage), &dev, page, subpage, verbose);
			let page = log_page::parse(&data);
			if let Some(page) = page {
				print!("{:?}\n", page);
				print!("{:#?}\n", page.parse_params());
			}
		}
	}
	*/
}
