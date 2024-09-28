use std::path::PathBuf;

use eframe::egui;
use egui_extras::{Column, TableBuilder};

mod bom;

fn main() {
    env_logger::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default(),
        ..Default::default()
    };

    _ = eframe::run_native("BOM compare", options, Box::new(|_| Box::<App>::default()));
}

#[derive(Default)]
struct App {
    bom: Option<bom::BomHandler>,
    error_message: String,
    path_1: String,
    path_2: String,
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("CTRL").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.monospace("BOM #1:");
                ui.text_edit_singleline(&mut self.path_1);

                if ui.button("Open").clicked() {
                    if let Some(file) = rfd::FileDialog::new()
                        .add_filter("XLSX", &["xlsx"])
                        .pick_file()
                    {
                        self.path_1 = file.display().to_string();
                    }
                }
            });

            ui.horizontal(|ui| {
                ui.monospace("BOM #2:");
                ui.text_edit_singleline(&mut self.path_2);

                if ui.button("Open").clicked() {
                    if let Some(file) = rfd::FileDialog::new()
                        .add_filter("XLSX", &["xlsx"])
                        .pick_file()
                    {
                        self.path_2 = file.display().to_string();
                    }
                }

                if ui.button("Compare").clicked()
                    && !self.path_1.is_empty()
                    && !self.path_2.is_empty()
                {
                    self.bom = None;
                    self.error_message.clear();

                    match bom::BomHandler::load(vec![
                        PathBuf::from(&self.path_1),
                        PathBuf::from(&self.path_2),
                    ]) {
                        Ok(new_bom) => self.bom = Some(new_bom),
                        Err(msg) => self.error_message = format!("{}", msg),
                    }
                }

                ui.set_enabled(false);
                if ui.button("Export").clicked() {
                    todo!();
                }
            });

            ui.separator();
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                if !self.error_message.is_empty() {
                    ui.monospace(&self.error_message);
                } else if let Some(boms) = &self.bom {
                    let diff = boms.get_diff();

                    TableBuilder::new(ui)
                        .striped(true)
                        .column(Column::initial(150.0).resizable(true))
                        .columns(Column::remainder().resizable(true), 2)
                        .body(|mut body| {
                            for (item, data) in diff.iter() {
                                body.row(28.0, |mut row| {
                                    row.col(|ui| {
                                        ui.separator();
                                        ui.monospace(item);
                                    });
                                    row.col(|ui| {
                                        ui.separator();
                                    });
                                    row.col(|ui| {
                                        ui.separator();
                                    });
                                });

                                for d in data {
                                    body.row(14.0, |mut row| {
                                        row.col(|ui| {
                                            ui.monospace(format!("\t{}", d[0]));
                                        });
                                        row.col(|ui| {
                                            ui.add(
                                                egui::Label::new(
                                                    egui::RichText::new(&d[1]).monospace(),
                                                )
                                                .truncate(true),
                                            );
                                        });
                                        row.col(|ui| {
                                            ui.add(
                                                egui::Label::new(
                                                    egui::RichText::new(&d[2]).monospace(),
                                                )
                                                .truncate(true),
                                            );
                                        });
                                    });
                                }
                            }
                        });
                }
            });
        });
    }
}
