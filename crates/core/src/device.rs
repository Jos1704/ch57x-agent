//! Detección del macro pad `1189:8890` (CH57x, 6 teclas + 1 perilla).

use anyhow::{Context, Result};
use rusb::{Device, Direction, GlobalContext, TransferType, UsbContext};

pub const VENDOR_ID: u16 = 0x1189;
pub const PRODUCT_ID: u16 = 0x8890;
/// Modelo de `ch57x-keyboard-tool` que corresponde a `1189:8890`.
pub const MODEL: &str = "ch57x-2";
pub const ROWS: u8 = 2;
pub const COLUMNS: u8 = 3;
pub const KNOBS: u8 = 1;
/// Interfaz HID por la que se programa el teclado (endpoint OUT `0x02`).
pub const PROGRAMMING_INTERFACE: u8 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    pub bus: u8,
    pub address: u8,
}

impl Found {
    pub fn usb_path(&self) -> String {
        format!("/dev/bus/usb/{:03}/{:03}", self.bus, self.address)
    }
}

pub fn find() -> Result<Vec<Found>> {
    find_in(&GlobalContext::default())
}

pub fn find_in<C: UsbContext>(ctx: &C) -> Result<Vec<Found>> {
    let mut found = Vec::new();
    for device in ctx.devices().context("listar dispositivos USB")?.iter() {
        let Ok(desc) = device.device_descriptor() else { continue };
        if desc.vendor_id() == VENDOR_ID && desc.product_id() == PRODUCT_ID {
            found.push(Found { bus: device.bus_number(), address: device.address() });
        }
    }
    Ok(found)
}

#[derive(Debug)]
pub struct Endpoint {
    pub address: u8,
    pub direction: &'static str,
    pub transfer: &'static str,
}

#[derive(Debug)]
pub struct Interface {
    pub number: u8,
    pub class: u8,
    pub protocol: &'static str,
    pub endpoints: Vec<Endpoint>,
}

#[derive(Debug)]
pub enum Access {
    Ok,
    Denied,
    Error(String),
}

#[derive(Debug)]
pub struct Report {
    pub device: Found,
    pub interfaces: Vec<Interface>,
    pub access: Access,
    /// La interfaz de programación tiene un endpoint IN, pero el protocolo
    /// conocido no define ningún comando para leer el perfil guardado.
    pub programming_endpoint_in: bool,
}

/// Inspecciona el dispositivo sin modificarlo.
pub fn describe(target: &Found) -> Result<Report> {
    let device = GlobalContext::default()
        .devices()?
        .iter()
        .find(|d| d.bus_number() == target.bus && d.address() == target.address)
        .context("el dispositivo ya no está conectado")?;
    let interfaces = interfaces(&device)?;
    let programming_endpoint_in = interfaces
        .iter()
        .filter(|i| i.number == PROGRAMMING_INTERFACE)
        .flat_map(|i| &i.endpoints)
        .any(|e| e.direction == "IN");
    let access = match device.open() {
        Ok(_) => Access::Ok,
        Err(rusb::Error::Access) => Access::Denied,
        Err(e) => Access::Error(e.to_string()),
    };
    Ok(Report { device: target.clone(), interfaces, access, programming_endpoint_in })
}

fn interfaces(device: &Device<GlobalContext>) -> Result<Vec<Interface>> {
    let config = device.config_descriptor(0).context("leer descriptor de configuración")?;
    let mut out = Vec::new();
    for iface in config.interfaces() {
        for desc in iface.descriptors() {
            let protocol = match (desc.class_code(), desc.protocol_code()) {
                (3, 1) => "teclado",
                (3, 2) => "ratón",
                (3, _) => "HID genérico",
                _ => "otro",
            };
            let endpoints = desc
                .endpoint_descriptors()
                .map(|ep| Endpoint {
                    address: ep.address(),
                    direction: match ep.direction() {
                        Direction::In => "IN",
                        Direction::Out => "OUT",
                    },
                    transfer: match ep.transfer_type() {
                        TransferType::Interrupt => "interrupt",
                        TransferType::Bulk => "bulk",
                        TransferType::Isochronous => "isochronous",
                        TransferType::Control => "control",
                    },
                })
                .collect();
            out.push(Interface {
                number: desc.interface_number(),
                class: desc.class_code(),
                protocol,
                endpoints,
            });
        }
    }
    Ok(out)
}
