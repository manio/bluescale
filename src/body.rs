use chrono::NaiveDateTime;
use std::fmt;

#[derive(Clone)]
pub struct Person {
    pub sex: u8, // male = 1; female = 0
    pub age: f32,
    pub height: f32,
}

impl fmt::Display for Person {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "sex: {}, age: {}, height: {} cm",
            {
                if self.sex == 1 {
                    "Male ♂️ "
                } else {
                    "Female ♀️ "
                }
            },
            self.age,
            self.height
        )?;
        Ok(())
    }
}

pub struct Measurement {
    pub date_time: NaiveDateTime,
    pub weight: f32,
    pub bmi: f32,
    pub water_rate: f32,
    pub bmr: f32,
    pub visceral_fat: f32,
    pub bf: f32,
    pub muscle_kg: f32,
    pub muscle_rate: f32,
    pub bone_mass: f32,
}

impl fmt::Display for Measurement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "    datetime: {}\n", self.date_time)?;
        write!(f, "    weight: {} kg\n", self.weight)?;
        write!(f, "    BMI: {}\n", self.bmi)?;
        write!(f, "    water: {} %\n", self.water_rate)?;
        write!(f, "    basal metabolism: {} kcal\n", self.bmr)?;
        write!(f, "    visceral fat: {} %\n", self.visceral_fat)?;
        write!(f, "    body fat: {} %\n", self.bf)?;
        write!(f, "    lean body mass: {} %\n", self.muscle_rate)?;
        write!(f, "    lean body mass: {} kg\n", self.muscle_kg)?;
        write!(f, "    bone mass: {} kg\n", self.bone_mass)?;
        Ok(())
    }
}

impl Person {
    pub fn get_lbm_coefficient(&self, weight: f32, impedance: f32) -> f32 {
        let mut lbm: f32 = (self.height * 9.058 / 100.0) * (self.height / 100.0);
        lbm += weight * 0.32 + 12.226;
        lbm -= impedance * 0.0068;
        lbm -= self.age * 0.0542;

        return lbm;
    }

    pub fn get_bmi(&self, weight: f32) -> f32 {
        return weight / (((self.height * self.height) / 100.0) / 100.0);
    }

    pub fn get_muscle(&self, weight: f32, impedance: f32) -> f32 {
        let mut muscle_mass: f32 = weight
            - ((self.get_body_fat(weight, impedance) * 0.01) * weight)
            - self.get_bone_mass(weight, impedance);

        if self.sex == 0 && muscle_mass >= 84.0 {
            muscle_mass = 120.0;
        } else if self.sex == 1 && muscle_mass >= 93.5 {
            muscle_mass = 120.0;
        }

        return muscle_mass;
    }

    pub fn get_water(&self, weight: f32, impedance: f32) -> f32 {
        let coeff: f32;
        let water: f32 = (100.0 - self.get_body_fat(weight, impedance)) * 0.7;

        if water < 50.0 {
            coeff = 1.02;
        } else {
            coeff = 0.98;
        }

        return coeff * water;
    }

    pub fn get_bone_mass(&self, weight: f32, impedance: f32) -> f32 {
        let mut bone_mass: f32;
        let base: f32;

        if self.sex == 0 {
            base = 0.245691014;
        } else {
            base = 0.18016894;
        }

        bone_mass = (base - (self.get_lbm_coefficient(weight, impedance) * 0.05158)) * -1.0;

        if bone_mass > 2.2 {
            bone_mass += 0.1;
        } else {
            bone_mass -= 0.1;
        }

        if self.sex == 0 && bone_mass > 5.1 {
            bone_mass = 8.0;
        } else if self.sex == 1 && bone_mass > 5.2 {
            bone_mass = 8.0;
        }

        return bone_mass;
    }

    pub fn get_visceral_fat(&self, weight: f32) -> f32 {
        let visceral_fat: f32;
        if self.sex == 0 {
            if weight > (13.0 - (self.height * 0.5)) * -1.0 {
                let subsubcalc: f32 =
                    ((self.height * 1.45) + (self.height * 0.1158) * self.height) - 120.0;
                let subcalc: f32 = weight * 500.0 / subsubcalc;
                visceral_fat = (subcalc - 6.0) + (self.age * 0.07);
            } else {
                let subcalc: f32 = 0.691 + (self.height * -0.0024) + (self.height * -0.0024);
                visceral_fat = (((self.height * 0.027) - (subcalc * weight)) * -1.0)
                    + (self.age * 0.07)
                    - self.age;
            }
        } else {
            if self.height < weight * 1.6 {
                let subcalc: f32 =
                    ((self.height * 0.4) - (self.height * (self.height * 0.0826))) * -1.0;
                visceral_fat = ((weight * 305.0) / (subcalc + 48.0)) - 2.9 + (self.age * 0.15);
            } else {
                let subcalc: f32 = 0.765 + self.height * -0.0015;
                visceral_fat =
                    (((self.height * 0.143) - (weight * subcalc)) * -1.0) + (self.age * 0.15) - 5.0;
            }
        }

        return visceral_fat;
    }

    pub fn get_body_fat(&self, weight: f32, impedance: f32) -> f32 {
        let mut body_fat: f32;
        let mut lbm_sub: f32 = 0.8;

        if self.sex == 0 && self.age <= 49.0 {
            lbm_sub = 9.25;
        } else if self.sex == 0 && self.age > 49.0 {
            lbm_sub = 7.25;
        }

        let lbm_coeff: f32 = self.get_lbm_coefficient(weight, impedance);
        let mut coeff: f32 = 1.0;

        if self.sex == 1 && weight < 61.0 {
            coeff = 0.98;
        } else if self.sex == 0 && weight > 60.0 {
            coeff = 0.96;

            if self.height > 160.0 {
                coeff *= 1.03;
            }
        } else if self.sex == 0 && weight < 50.0 {
            coeff = 1.02;

            if self.height > 160.0 {
                coeff *= 1.03;
            }
        }

        body_fat = (1.0 - (((lbm_coeff - lbm_sub) * coeff) / weight)) * 100.0;

        if body_fat > 63.0 {
            body_fat = 75.0;
        }

        return body_fat;
    }

    pub fn get_bmr(&self, weight: f32) -> f32 {
        let mut bmr: f32;

        if self.sex == 0 {
            //female
            bmr = 864.6 + weight * 10.2036;
            bmr -= self.height * 0.39336;
            bmr -= self.age * 6.204;
        } else {
            bmr = 877.8 + weight * 14.916;
            bmr -= self.height * 0.726;
            bmr -= self.age * 8.976;
        }

        return bmr;
    }
}
