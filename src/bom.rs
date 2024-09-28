use log::{debug, info};
use std::path::{Path, PathBuf};

use umya_spreadsheet::*;

#[derive(Default)]
struct RawBom {
    items: Vec<(String, BomItemData)>, // Item_No, and BomItemData
}

impl RawBom {
    fn load(path: &Path) -> anyhow::Result<RawBom> {
        debug!("RawBom::load - path:{:?}", path);

        let mut ret = RawBom::default();

        let book = reader::xlsx::read(path)?;
        let sheet = book.get_sheet(&0).unwrap();

        let mut row = 20;
        loop {
            let find_no = sheet.get_value((1, row)); // (col, row)
            if find_no.is_empty() {
                break;
            }

            if find_no == "OD" {
                let name = sheet.get_value((4, row));
                let manufacturer = sheet.get_value((11, row));
                let order_desc = sheet.get_value((15, row));

                if let Some(item) = ret.items.last_mut() {
                    item.1.order_data.push(BomOrderData {
                        name,
                        manufacturer,
                        order_desc,
                    });
                }
            } else {
                let item_no = sheet.get_value((2, row));
                let rev = sheet.get_value((3, row));
                let name = sheet.get_value((4, row));
                let quantity_text = sheet.get_value((5, row));
                let additional_name = sheet.get_value((17, row));
                let short_desc = sheet.get_value((18, row));
                let ref_designator = sheet.get_value((20, row));

                let quantity = if let Ok(x) = quantity_text.parse::<i32>() {
                    x
                } else if let Ok(y) = quantity_text.parse::<f32>() {
                    y as i32
                } else {
                    0
                };

                ret.items.push((
                    item_no,
                    BomItemData {
                        rev,
                        name,
                        quantity,
                        short_desc,
                        additional_name,
                        ref_designator,
                        order_data: Vec::new(),
                    },
                ))
            }

            row += 1;
        }

        Ok(ret)
    }
}

#[derive(Default)]
pub struct BomHandler {
    paths: Vec<PathBuf>,
    boms: usize,
    items: Vec<BomItem>,
	diff: Vec<(String, Vec<Vec<String>>)>,
}

impl BomHandler {
    pub fn load(paths: Vec<PathBuf>) -> anyhow::Result<BomHandler> {
        debug!("BomHandler::load - paths:{:?}", paths);
        let mut ret = BomHandler::default();
        ret.paths = paths;
        ret.boms = ret.paths.len();

        for (index, path) in ret.paths.iter().enumerate() {
            let raw = RawBom::load(path)?;
            for (item, data) in raw.items {
                if let Some(dst) = ret.items.iter_mut().find(|f| f.item_no == item) {
                    dst.push(index, data);
                } else {
                    ret.items.push(BomItem::new(index, ret.boms, item, data));
                }
            }
        }

		ret.sort();
		ret.generate_diff();

        Ok(ret)
    }

	fn sort(&mut self) {
		self.items.sort_by(|a,b| a.item_no.cmp(&b.item_no));

		for item in self.items.iter_mut() {
			for bom in item.boms.iter_mut().flatten() {
				bom.order_data.sort_by(|a,b| a.order_desc.cmp(&b.order_desc));
			}
		}
	}

	fn generate_diff(&mut self) {
		self.diff.clear();

		for item in self.items.iter() {
			let diff = item.generate_diff();
			if !diff.is_empty() {
				self.diff.push((item.item_no.clone(), diff));
			}
		}
	}

    pub fn get_diff(&self) -> &Vec<(String, Vec<Vec<String>>)> {
        &self.diff
    }
}

pub struct BomItem {
    item_no: String,
    boms: Vec<Option<BomItemData>>,
}

impl BomItem {
    fn new(index: usize, capacity: usize, item: String, data: BomItemData) -> Self {
        let mut boms = vec![None; capacity];
        boms[index] = Some(data);

        BomItem {
            item_no: item,
            boms,
        }
    }

    fn push(&mut self, index: usize, data: BomItemData) {
        self.boms[index] = Some(data);
    }

    fn matches(&self) -> bool {
        // if any are None, then return false
        if self.boms.contains(&None) {
            return false;
        }

        for bom in self.boms.iter().skip(1) {
            if self.boms[0] != *bom {
                return false;
            }
        }

        true
    }

    fn generate_diff(&self) -> Vec<Vec<String>> {
        let mut ret = Vec::new();

        if self.boms.contains(&None) {
			// if for one of the BOMs the item is None, then we have to populate every field
            ret.push(vec!["Rev".to_string()]);
            ret.push(vec!["Name".to_string()]);
            ret.push(vec!["Quantity".to_string()]);
            ret.push(vec!["Additional name".to_string()]);
            ret.push(vec!["Short desc.".to_string()]);
            ret.push(vec!["Ref. designator".to_string()]);
            ret.push(vec!["Order data".to_string()]);

            for bom in self.boms.iter() {
                if let Some(b) = bom {
                    ret[0].push(b.rev.clone());
                    ret[1].push(b.name.clone());
                    ret[2].push(b.quantity.to_string());
                    ret[3].push(b.additional_name.clone());
                    ret[4].push(b.short_desc.clone());
                    ret[5].push(b.ref_designator.clone());

                    let od = b.get_mpn_list().join(", ");
                    ret[6].push(od)
                } else {
                    for v in ret.iter_mut() {
                        v.push(String::new());
                    }
                }
            }
        } else {
			// if not, then check if the values are different, and only store if they are
			let boms = self.boms.iter().filter_map(Option::as_ref).collect::<Vec<_>>();
			let boms_0 = boms[0];
			
			// First check which fields are different
			let mut diff_tracker = [false;7];
			for bom in boms.iter().skip(1) {
				if bom.rev != boms_0.rev {
					diff_tracker[0] = true;
				}
				if bom.name != boms_0.name {
					diff_tracker[1] = true;
				}
				if bom.quantity != boms_0.quantity {
					diff_tracker[2] = true;
				}
				if bom.additional_name != boms_0.additional_name {
					diff_tracker[3] = true;
				}
				if bom.short_desc != boms_0.short_desc {
					diff_tracker[4] = true;
				}
				if bom.ref_designator != boms_0.ref_designator {
					diff_tracker[5] = true;
				}
				if bom.order_data != boms_0.order_data {
					diff_tracker[6] = true;
				}
			}

			let mut i2 = 0;
			for (i,dt) in diff_tracker.into_iter().enumerate().skip(1) // skipping Rev
			{
				if dt {
					ret.push(vec![match i {
						0 => "Rev".to_string(),
						1 => "Name".to_string(),
						2 => "Quantity".to_string(),
						3 => "Additional name".to_string(),
						4 => "Short desc.".to_string(),
						5 => "Ref. designator".to_string(),
						6 => "Order data".to_string(),
						_ => panic!()

					}]);

					for bom in &boms {
						ret[i2].push(match i {
							0 => bom.rev.clone(),
							1 => bom.name.clone(),
							2 => bom.quantity.to_string(),
							3 => bom.additional_name.clone(),
							4 => bom.short_desc.clone(),
							5 => bom.ref_designator.clone(),
							6 => bom.get_mpn_list().join(", "),
							_ => panic!()
	
						});
					}
					i2 += 1;
				}
			}
        }

        ret
    }

    pub fn get_item_no(&self) -> &str {
        &self.item_no
    }

    pub fn get_item_data(&self, index: usize) -> Option<&BomItemData> {
        if let Some(Some(x)) = self.boms.get(index) {
            Some(x)
        } else {
            None
        }
    }
}

#[derive(PartialEq, Eq, Clone)]
pub struct BomItemData {
    rev: String, // I'm not sure if it is always a int or not.
    name: String,
    quantity: i32, // Could get away without conversion too, and just use it as a String
    additional_name: String,
    short_desc: String,
    ref_designator: String, //Could split it up to individual positions
    order_data: Vec<BomOrderData>,
}

impl BomItemData {
    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_quantity(&self) -> i32 {
        self.quantity
    }

	pub fn get_mpn_list(&self) -> Vec<&str> {
		self.order_data.iter().map(|f| f.order_desc.as_str()).collect()
	}
}

#[derive(PartialEq, Eq, Clone)]
pub struct BomOrderData {
    name: String,
    manufacturer: String,
    order_desc: String, // MPN
}
