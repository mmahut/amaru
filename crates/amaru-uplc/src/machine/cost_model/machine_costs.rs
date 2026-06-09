// Copyright 2025 PRAGMA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::machine::{ExBudget, cost_model::cost_map::CostMap};

#[derive(Debug, PartialEq)]
pub struct MachineCosts([ExBudget; 9]);

impl Default for MachineCosts {
    fn default() -> Self {
        Self::new()
    }
}

impl MachineCosts {
    pub fn new() -> Self {
        MachineCosts([
            ExBudget::constant(),
            ExBudget::var(),
            ExBudget::lambda(),
            ExBudget::apply(),
            ExBudget::delay(),
            ExBudget::force(),
            ExBudget::builtin(),
            ExBudget::constr(),
            ExBudget::case(),
        ])
    }

    pub fn get(&self, index: usize) -> ExBudget {
        self.0[index]
    }

    pub fn initialize_machine_costs(cost_map: &CostMap) -> Self {
        MachineCosts([
            ExBudget::new(cost_map["cek_const_cost-exBudgetmem"], cost_map["cek_const_cost-exBudgetCPU"]),
            ExBudget::new(cost_map["cek_var_cost-exBudgetmem"], cost_map["cek_var_cost-exBudgetCPU"]),
            ExBudget::new(cost_map["cek_lam_cost-exBudgetmem"], cost_map["cek_lam_cost-exBudgetCPU"]),
            ExBudget::new(cost_map["cek_apply_cost-exBudgetmem"], cost_map["cek_apply_cost-exBudgetCPU"]),
            ExBudget::new(cost_map["cek_delay_cost-exBudgetmem"], cost_map["cek_delay_cost-exBudgetCPU"]),
            ExBudget::new(cost_map["cek_force_cost-exBudgetmem"], cost_map["cek_force_cost-exBudgetCPU"]),
            ExBudget::new(cost_map["cek_builtin_cost-exBudgetmem"], cost_map["cek_builtin_cost-exBudgetCPU"]),
            ExBudget::new(cost_map["cek_constr_cost-exBudgetmem"], cost_map["cek_constr_cost-exBudgetCPU"]),
            ExBudget::new(cost_map["cek_case_cost-exBudgetmem"], cost_map["cek_case_cost-exBudgetCPU"]),
        ])
    }
}
