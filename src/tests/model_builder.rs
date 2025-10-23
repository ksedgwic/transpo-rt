// Minimal in-tree reimplementation of the transit_model_builder helpers used by our tests.
use transit_model::model::{Collections, Model};
use transit_model::objects::{Calendar, Route, StopPoint, StopTime, Time, VehicleJourney};
use typed_index_collection::Idx;

#[derive(Default)]
pub struct ModelBuilder {
    collections: Collections,
}

pub struct VehicleJourneyBuilder<'a> {
    model: &'a mut ModelBuilder,
    vj_idx: Idx<VehicleJourney>,
}

pub trait IntoTime {
    fn into_time(self) -> Time;
}

impl IntoTime for Time {
    fn into_time(self) -> Time {
        self
    }
}

impl IntoTime for &Time {
    fn into_time(self) -> Time {
        *self
    }
}

impl IntoTime for &str {
    fn into_time(self) -> Time {
        self.parse().unwrap()
    }
}

impl ModelBuilder {
    pub fn vj<F>(mut self, name: &str, mut vj_initer: F) -> Self
    where
        F: FnMut(VehicleJourneyBuilder),
    {
        let mut new_vj = VehicleJourney::default();
        new_vj.id = name.into();
        let vj_idx = self
            .collections
            .vehicle_journeys
            .push(new_vj)
            .expect(&format!("vj {} already exists", name));
        let vj_builder = VehicleJourneyBuilder {
            model: &mut self,
            vj_idx,
        };

        vj_initer(vj_builder);
        self
    }

    pub fn route<F>(mut self, id: &str, mut route_initer: F) -> Self
    where
        F: FnMut(&mut Route),
    {
        self.collections.routes.get_or_create_with(id, || {
            let mut route = Route::default();
            route_initer(&mut route);
            route
        });
        self
    }

    pub fn calendar<F>(mut self, id: &str, mut calendar_initer: F) -> Self
    where
        F: FnMut(&mut Calendar),
    {
        self.collections.calendars.get_or_create_with(id, || {
            let mut calendar = Calendar::default();
            calendar_initer(&mut calendar);
            calendar
        });
        self
    }

    pub fn build(self) -> Model {
        Model::new(self.collections).unwrap()
    }
}

impl<'a> VehicleJourneyBuilder<'a> {
    fn find_or_create_sp(&mut self, sp: &str) -> Idx<StopPoint> {
        self.model
            .collections
            .stop_points
            .get_idx(sp)
            .unwrap_or_else(|| {
                let sa_id = format!("sa:{}", sp);
                let new_sp = StopPoint {
                    id: sp.to_owned(),
                    name: sp.to_owned(),
                    stop_area_id: sa_id.clone(),
                    ..Default::default()
                };

                self.model.collections.stop_areas.get_or_create(&sa_id);

                self.model
                    .collections
                    .stop_points
                    .push(new_sp)
                    .expect(&format!("stoppoint {} already exists", sp))
            })
    }

    pub fn st(mut self, name: &str, arrival: impl IntoTime, departure: impl IntoTime) -> Self {
        let stop_point_idx = self.find_or_create_sp(name);
        {
            let vj = &mut self
                .model
                .collections
                .vehicle_journeys
                .index_mut(self.vj_idx);
            let sequence = vj.stop_times.len() as u32;
            let stop_time = StopTime {
                stop_point_idx,
                sequence,
                arrival_time: arrival.into_time(),
                departure_time: departure.into_time(),
                boarding_duration: 0,
                alighting_duration: 0,
                pickup_type: 0,
                drop_off_type: 0,
                local_zone_id: None,
                precision: None,
            };
            vj.stop_times.push(stop_time);
        }

        self
    }

    pub fn route(self, id: &str) -> Self {
        {
            let vj = &mut self
                .model
                .collections
                .vehicle_journeys
                .index_mut(self.vj_idx);
            vj.route_id = id.to_owned();
        }
        self
    }

    pub fn calendar(self, id: &str) -> Self {
        {
            let vj = &mut self
                .model
                .collections
                .vehicle_journeys
                .index_mut(self.vj_idx);
            vj.service_id = id.to_owned();
        }
        self
    }
}

impl<'a> Drop for VehicleJourneyBuilder<'a> {
    fn drop(&mut self) {
        let collections = &mut self.model.collections;
        let new_vj = &collections.vehicle_journeys[self.vj_idx];

        let dataset_contributor = {
            let mut dataset = collections.datasets.get_or_create(&new_vj.dataset_id);
            dataset.start_date = chrono::NaiveDate::from_ymd(1970, 1, 1);
            dataset.end_date = chrono::NaiveDate::from_ymd(2100, 1, 1);
            dataset.contributor_id.clone()
        };
        collections
            .contributors
            .get_or_create(&dataset_contributor);

        collections.companies.get_or_create(&new_vj.company_id);
        {
            let mut calendar = collections.calendars.get_or_create(&new_vj.service_id);
            if calendar.dates.is_empty() {
                calendar
                    .dates
                    .insert(chrono::NaiveDate::from_ymd(1970, 1, 1));
            }
        }
        collections
            .physical_modes
            .get_or_create(&new_vj.physical_mode_id);

        let route_line = {
            let route = collections.routes.get_or_create(&new_vj.route_id);
            route.line_id.clone()
        };
        let line_mode_network = {
            let line = collections.lines.get_or_create(&route_line);
            (line.commercial_mode_id.clone(), line.network_id.clone())
        };
        collections
            .commercial_modes
            .get_or_create(&line_mode_network.0);
        collections
            .networks
            .get_or_create(&line_mode_network.1);
    }
}
