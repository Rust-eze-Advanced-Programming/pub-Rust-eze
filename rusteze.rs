
#[allow(unused)]
pub mod rust_eze {
    use std::collections::{HashMap, HashSet};
    use crossbeam_channel;
    use std::thread;
    use crossbeam_channel::{Receiver, Sender};
    use crate::components::asteroid::Asteroid;
    use crate::components::planet::{Planet, PlanetAI, PlanetState, PlanetType};
    use crate::components::resource::*;
    use crate::components::rocket::Rocket;
    use crate::protocols::messages::{ExplorerToPlanet, OrchestratorToPlanet, PlanetToExplorer, PlanetToOrchestrator};

    struct RustEzeAi {
        running: bool,
        killed: bool,
    }
    impl PlanetAI for RustEzeAi {
        fn handle_orchestrator_msg(&mut self, state: &mut PlanetState, generator: &Generator, combinator: &Combinator, msg: OrchestratorToPlanet) -> Option<PlanetToOrchestrator> {
            if (self.running && !self.killed) {
                match msg {
                    OrchestratorToPlanet::Sunray(sunray) => {
                        if let Some(cell) = state.cells_iter_mut().next() {
                            cell.charge(sunray);
                        }
                        let response = PlanetToOrchestrator::SunrayAck { planet_id: state.id() };
                        Some(response)
                    },
                    OrchestratorToPlanet::InternalStateRequest => {
                        let response = PlanetToOrchestrator::InternalStateResponse { planet_id: state.id(), planet_state: state.to_dummy() };
                        Some(response)
                    },

                    OrchestratorToPlanet::KillPlanet=>{
                        self.killed=true;
                        let res= PlanetToOrchestrator::KillPlanetResult { planet_id: state.id(), };
                        Some(res)
                    }

                    //non si arriva mai a _ per la logica di planet.run( )
                    _ => { None }
                }
            } else if(!self.killed){ Some(PlanetToOrchestrator::Stopped { planet_id: state.id(), }) }
            else{ None }
        }


        fn handle_explorer_msg(&mut self, state: &mut PlanetState, generator: &Generator, combinator: &Combinator, msg: ExplorerToPlanet) -> Option<PlanetToExplorer> {
            if(self.running && !self.killed) {
                match msg {
                    ExplorerToPlanet::AvailableEnergyCellRequest { explorer_id } => {
                        let available_cells = state.cells_count() as u32;
                        Some(PlanetToExplorer::AvailableEnergyCellResponse { available_cells })
                    },
                    ExplorerToPlanet::CombineResourceRequest { explorer_id, msg } => {
                        let complex_response: Result<ComplexResource, (String, GenericResource, GenericResource)> =
                            match msg {
                                // Water from Hydrogen + Oxygen (both basic)
                                ComplexResourceRequest::Water(h, o) => Err((
                                    String::from("Planet does not support combinations"),
                                    GenericResource::BasicResources(BasicResource::Hydrogen(h)),
                                    GenericResource::BasicResources(BasicResource::Oxygen(o)),
                                )),

                                // Diamond from Carbon + Carbon (both basic)
                                ComplexResourceRequest::Diamond(c1, c2) => Err((
                                    String::from("Planet does not support combinations"),
                                    GenericResource::BasicResources(BasicResource::Carbon(c1)),
                                    GenericResource::BasicResources(BasicResource::Carbon(c2)),
                                )),

                                // Life from Water + Carbon (left complex, right basic)
                                ComplexResourceRequest::Life(w, c) => Err((
                                    String::from("Planet does not support combinations"),
                                    GenericResource::ComplexResources(ComplexResource::Water(w)),
                                    GenericResource::BasicResources(BasicResource::Carbon(c)),
                                )),

                                // Robot from Silicon + Life (left basic, right complex)
                                ComplexResourceRequest::Robot(s, l) => Err((
                                    String::from("Planet does not support combinations"),
                                    GenericResource::BasicResources(BasicResource::Silicon(s)),
                                    GenericResource::ComplexResources(ComplexResource::Life(l)),
                                )),

                                // Dolphin from Water + Life (both complex)
                                ComplexResourceRequest::Dolphin(w, l) => Err((
                                    String::from("Planet does not support combinations"),
                                    GenericResource::ComplexResources(ComplexResource::Water(w)),
                                    GenericResource::ComplexResources(ComplexResource::Life(l)),
                                )),

                                // AIPartner from Robot + Diamond (both complex)
                                ComplexResourceRequest::AIPartner(r, d) => Err((
                                    String::from("Planet does not support combinations"),
                                    GenericResource::ComplexResources(ComplexResource::Robot(r)),
                                    GenericResource::ComplexResources(ComplexResource::Diamond(d)),
                                )),
                            };

                        Some(PlanetToExplorer::CombineResourceResponse { complex_response })
                    },
                    ExplorerToPlanet::GenerateResourceRequest { explorer_id, resource } => {
                        if state.cells_count() == 0 {
                            Some(PlanetToExplorer::GenerateResourceResponse { resource: None })
                        } else {
                            let cell = state.cell_mut(0);
                            let resource = Some(match resource {
                                BasicResourceType::Carbon   => BasicResource::Carbon(generator.make_carbon(cell).unwrap()),
                                BasicResourceType::Silicon  => BasicResource::Silicon(generator.make_silicon(cell).unwrap()),
                                BasicResourceType::Hydrogen => BasicResource::Hydrogen(generator.make_hydrogen(cell).unwrap()),
                                BasicResourceType::Oxygen   => BasicResource::Oxygen(generator.make_oxygen(cell).unwrap()),
                            });
                            Some(PlanetToExplorer::GenerateResourceResponse { resource })
                        }
                    },
                    ExplorerToPlanet::SupportedCombinationRequest { explorer_id } => {
                        Some(PlanetToExplorer::SupportedCombinationResponse { combination_list: combinator.all_available_recipes() })
                    },
                    ExplorerToPlanet::SupportedResourceRequest { explorer_id } => {
                        Some(PlanetToExplorer::SupportedResourceResponse { resource_list: generator.all_available_recipes(), })
                    },
                }
            } else if !self.killed{
                Some(PlanetToExplorer::Stopped{})
            }
            else{
                None
            }
        }

        fn handle_asteroid(&mut self, state: &mut PlanetState, generator: &Generator, combinator: &Combinator) -> Option<Rocket> {
            if(self.running) {
                if state.has_rocket() { state.take_rocket() }
                else {
                    None
                }
            }else{
                None
            }
        }

        fn start(&mut self, state: &PlanetState) {
            if (!self.running) {
                self.running = true;
            }
        }

        fn stop(&mut self, state: &PlanetState) {
            if (self.running) {
                self.running = false;
            }
        }
    }

    //function to create rust_eze planet
    pub fn createplanet(
        from_orchestrator: Receiver<OrchestratorToPlanet>,
        to_orchestrator: Sender<PlanetToOrchestrator>,
        from_explorers: Receiver<ExplorerToPlanet>,
        planet_id: u32
    ) -> Planet {
        let rust_eze_ai = Box::new(RustEzeAi { running: false,killed:false });
        let rust_eze_type = PlanetType::D;
        let gen_rules: Vec<BasicResourceType> = vec![BasicResourceType::Carbon, BasicResourceType::Silicon ];
        let comb_rules: Vec<ComplexResourceType> = vec![];
        let orchestrator_channels = (from_orchestrator,to_orchestrator);

        let rust_eze = Planet::new(
            1,
            rust_eze_type,
            rust_eze_ai,
            gen_rules,
            comb_rules,
            orchestrator_channels,
            from_explorers,
        );
        if (rust_eze.is_ok()) {
            rust_eze.unwrap()
        } else {
            let err = String::from("Rust eze Planet creation failed");
        }
    }
}

#[cfg(test)]
#[allow(unused)]
mod tests {
    use std::collections::{HashMap, HashSet};
    use crossbeam_channel;
    use std::thread;
    use std::thread::sleep;
    use std::time::Duration;
    use crossbeam_channel::{unbounded, Receiver, Sender};
    use crate::components::asteroid::Asteroid;
    use crate::components::energy_cell::EnergyCell;
    use crate::components::forge::Forge;
    use crate::components::orchestrator;
    use crate::components::planet::{Planet, PlanetAI, PlanetState, PlanetType};
    use crate::components::resource::*;
    use crate::components::resource::BasicResource::Oxygen;
    use crate::components::rocket::Rocket;
    use crate::components::sunray::Sunray;
    use crate::logging::ActorType::Orchestrator;
    use crate::planets::rusteze::rust_eze::get_rust_eze;
    use crate::protocols::messages::{ExplorerToPlanet, OrchestratorToPlanet, PlanetToExplorer, PlanetToOrchestrator};
    use std::sync::OnceLock;
    static FORGE: OnceLock<Forge> = OnceLock::new();

    fn get_forge() -> &'static Forge {
        FORGE.get_or_init(|| Forge::new().unwrap())
    }

    pub struct MockOrchestrator {
        pub orchestrator_to_planet: Sender<OrchestratorToPlanet>,
        pub planet_to_orchestrator: Receiver<PlanetToOrchestrator>,
    }
    pub struct Mockforge{
        pub forge: Forge,
    }

    pub struct MockExplorer {
        pub explorer_to_planet: Sender<ExplorerToPlanet>,
        pub planet_to_explorer: Receiver<PlanetToExplorer>,
    }


    #[test]
    fn test_new_rust_eze() {
        //creating all channel
        let (orchestrator_to_planet_tx, orchestrator_to_planet_rx) =
            unbounded::<OrchestratorToPlanet>();

        let (planet_to_orchestrator_tx, planet_to_orchestrator_rx) =
            unbounded::<PlanetToOrchestrator>();

        let (explorers_to_planet_tx, explorers_to_planet_rx) =
            unbounded::<ExplorerToPlanet>();
        let (planet_to_explorer_tx, planet_to_explorer_rx) =
        unbounded::<PlanetToExplorer>();

        let orchestrator= MockOrchestrator{orchestrator_to_planet:orchestrator_to_planet_tx,planet_to_orchestrator:planet_to_orchestrator_rx};
        let explorer=MockExplorer{explorer_to_planet:explorers_to_planet_tx,planet_to_explorer:planet_to_explorer_rx};
        let rustezewrap=get_rust_eze(orchestrator_to_planet_rx,planet_to_orchestrator_tx,explorers_to_planet_rx);
        assert!(rustezewrap.is_ok());
        if(rustezewrap.is_ok()) {
            let rusteze=rustezewrap.unwrap();
        }

    }
    // --- Helper to get a charged cell ---
    fn get_charged_cell() -> EnergyCell {
        let mut cell = EnergyCell::new();
        // We use the real Sunray constructor now
        cell.charge(Sunray::new());
        cell
    }
    //helper for the test
    fn setup_test() -> (MockOrchestrator, MockExplorer, Sender<PlanetToExplorer>) {
        let (orchestrator_to_planet_tx, orchestrator_to_planet_rx) =
            unbounded::<OrchestratorToPlanet>();

        let (planet_to_orchestrator_tx, planet_to_orchestrator_rx) =
            unbounded::<PlanetToOrchestrator>();

        let (explorers_to_planet_tx, explorers_to_planet_rx) =
            unbounded::<ExplorerToPlanet>();
        let (planet_to_explorer_tx, planet_to_explorer_rx) =
            unbounded::<PlanetToExplorer>();

        let orchestrator= MockOrchestrator{orchestrator_to_planet:orchestrator_to_planet_tx,planet_to_orchestrator:planet_to_orchestrator_rx};
        let explorer=MockExplorer{explorer_to_planet:explorers_to_planet_tx,planet_to_explorer:planet_to_explorer_rx};
        let rustezewrap=get_rust_eze(orchestrator_to_planet_rx,planet_to_orchestrator_tx,explorers_to_planet_rx);
        let mut rusteze=rustezewrap.unwrap();
        let handle = thread::spawn(move || {
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let res = rusteze.run();
                match res {
                    Ok(_) => {}
                    Err(err) => {
                        dbg!(err);
                    }
                }
            }));
        });

        (orchestrator, explorer, planet_to_explorer_tx)
    }
    #[test]
    fn test_planet_ai_stopped() {
        //creating all channel
        let (orchestrator, explorer, snd_p_to_e) =setup_test();
        //1.verify the stopped state
        orchestrator.orchestrator_to_planet
            .send(OrchestratorToPlanet::InternalStateRequest {  })
            .unwrap();
        match orchestrator.planet_to_orchestrator.recv_timeout(Duration::from_millis(300)) {
            Ok(PlanetToOrchestrator::Stopped {planet_id} )=>assert_eq!(planet_id, 1),
            Ok(_)=>panic!("Planet responded while stopped"),
            Err(_)=>panic!("Timeout waiting for stopped planet"),
        }
        thread::sleep(Duration::from_millis(50));

        //2.verify correct response to startAI
        orchestrator.orchestrator_to_planet
            .send(OrchestratorToPlanet::StartPlanetAI {  })
            .unwrap();
        match orchestrator.planet_to_orchestrator.recv_timeout(Duration::from_millis(300)) {
            Ok(PlanetToOrchestrator::StartPlanetAIResult {planet_id} )=>assert_eq!(planet_id, 1),
            Ok(_)=>panic!("Planet responded while stopped"),
            Err(_)=>panic!("Timeout waiting for stopped planet"),
        }
    }

        #[test]
        fn test_planet_ai_start() {
            //creating all channel
            let (orchestrator, explorer, snd_p_to_e) = setup_test();

            //verify corret response to startAI
            orchestrator.orchestrator_to_planet
                .send(OrchestratorToPlanet::StartPlanetAI {})
                .unwrap();
            match orchestrator.planet_to_orchestrator.recv_timeout(Duration::from_millis(300)) {
                Ok(PlanetToOrchestrator::StartPlanetAIResult { planet_id }) => assert_eq!(planet_id, 1),
                Ok(_) => panic!("wrong ACK!"),
                Err(_) => panic!("Timeout waiting for planet"),
            }
        }

    #[test]
    fn test_planet_ai_int_req() {
        //creating all channel
        let (orchestrator, explorer, snd_p_to_e) = setup_test();

        //verify corret response to startAI
        orchestrator.orchestrator_to_planet
            .send(OrchestratorToPlanet::StartPlanetAI {})
            .unwrap();
        match orchestrator.planet_to_orchestrator.recv_timeout(Duration::from_millis(300)) {
            Ok(PlanetToOrchestrator::StartPlanetAIResult { planet_id }) => assert_eq!(planet_id, 1),
            Ok(_) => panic!("wrong ACK!"),
            Err(_) => panic!("Timeout waiting for planet"),
        }
        orchestrator.orchestrator_to_planet
            .send(OrchestratorToPlanet::InternalStateRequest {})
            .unwrap();
        match orchestrator.planet_to_orchestrator.recv_timeout(Duration::from_millis(300)) {
            Ok(PlanetToOrchestrator::InternalStateResponse {planet_id, planet_state }) => {assert_eq!(planet_id, 1)},
            Ok(_) => panic!("wrong ACK!"),
            Err(_) => panic!("Timeout waiting for planet"),
        }
    }
    #[test]
    fn test_planet_ai_stop() {
        //creating all channel
        let (orchestrator, explorer, snd_p_to_e) = setup_test();

        //verify corret response to startAI
        orchestrator.orchestrator_to_planet
            .send(OrchestratorToPlanet::StartPlanetAI {})
            .unwrap();
        match orchestrator.planet_to_orchestrator.recv_timeout(Duration::from_millis(300)) {
            Ok(PlanetToOrchestrator::StartPlanetAIResult { planet_id }) => assert_eq!(planet_id, 1),
            Ok(_) => panic!("wrong ACK!"),
            Err(_) => panic!("Timeout waiting for planet"),
        }
        orchestrator.orchestrator_to_planet
            .send(OrchestratorToPlanet::StopPlanetAI {})
            .unwrap();
        match orchestrator.planet_to_orchestrator.recv_timeout(Duration::from_millis(300)) {
            Ok(PlanetToOrchestrator::StopPlanetAIResult { planet_id }) => { assert_eq!(planet_id, 1) },
            Ok(_) => panic!("wrong ACK!"),
            Err(_) => panic!("Timeout waiting for planet"),
        }

        //verifying that the AI is stopped
        orchestrator.orchestrator_to_planet
            .send(OrchestratorToPlanet::InternalStateRequest {})
            .unwrap();
        match orchestrator.planet_to_orchestrator.recv_timeout(Duration::from_millis(300)) {
            Ok(PlanetToOrchestrator::Stopped { planet_id }) => { assert_eq!(planet_id, 1) },
            Ok(_) => panic!("wrong ACK!"),
            Err(_) => panic!("Timeout waiting for planet"),
        }
    }

    #[test]
    fn test_planet_ai_kill() {
        //creating all channel
        let (orchestrator, explorer, snd_p_to_e) = setup_test();
        //verify corret response to startAI
        orchestrator.orchestrator_to_planet
            .send(OrchestratorToPlanet::StartPlanetAI {})
            .unwrap();
        match orchestrator.planet_to_orchestrator.recv_timeout(Duration::from_millis(300)) {
            Ok(PlanetToOrchestrator::StartPlanetAIResult { planet_id }) => assert_eq!(planet_id, 1),
            Ok(_) => panic!("wrong ACK!"),
            Err(_) => panic!("Timeout waiting for planet"),
        }
        orchestrator.orchestrator_to_planet
            .send(OrchestratorToPlanet::KillPlanet {})
            .unwrap();
        match orchestrator.planet_to_orchestrator.recv_timeout(Duration::from_millis(300)) {
            Ok(PlanetToOrchestrator::KillPlanetResult { planet_id }) => { assert_eq!(planet_id, 1) },
            Ok(_) => panic!("wrong ACK!"),
            Err(_) => panic!("Timeout waiting for planet"),
        }
    }
    #[test]
    fn test_planet_ai_() {
        //creating all channel
        let (orchestrator, explorer, snd_p_to_e) = setup_test();
        //verify corret response to startAI
        orchestrator.orchestrator_to_planet
            .send(OrchestratorToPlanet::StartPlanetAI {})
            .unwrap();
        match orchestrator.planet_to_orchestrator.recv_timeout(Duration::from_millis(300)) {
            Ok(PlanetToOrchestrator::StartPlanetAIResult { planet_id }) => assert_eq!(planet_id, 1),
            Ok(_) => panic!("wrong ACK!"),
            Err(_) => panic!("Timeout waiting for planet"),
        }
        orchestrator.orchestrator_to_planet
            .send(OrchestratorToPlanet::KillPlanet {})
            .unwrap();
        match orchestrator.planet_to_orchestrator.recv_timeout(Duration::from_millis(300)) {
            Ok(PlanetToOrchestrator::KillPlanetResult { planet_id }) => { assert_eq!(planet_id, 1) },
            Ok(_) => panic!("wrong ACK!"),
            Err(_) => panic!("Timeout waiting for planet"),
        }
    }


    #[test]
    fn test_correct_basic_resource_generation() {
        //creating all channel
        let (orchestrator, explorer, snd_p_to_e) = setup_test();
        let forge=get_forge();
        //verify corret response to startAI
        orchestrator.orchestrator_to_planet
            .send(OrchestratorToPlanet::StartPlanetAI {})
            .unwrap();

        orchestrator.orchestrator_to_planet.send(OrchestratorToPlanet::IncomingExplorerRequest { explorer_id: 100, new_mpsc_sender: snd_p_to_e }).unwrap();
        orchestrator.orchestrator_to_planet.send(OrchestratorToPlanet::Sunray { 0: forge.generate_sunray() }).unwrap();
        explorer.explorer_to_planet.send(ExplorerToPlanet::GenerateResourceRequest { explorer_id: 100, resource: BasicResourceType::Carbon }).unwrap();

        match explorer.planet_to_explorer.recv_timeout(Duration::from_millis(300)) {
            Ok(PlanetToExplorer::GenerateResourceResponse { resource }) => assert_eq!(resource.unwrap().get_type(), BasicResourceType::Carbon),
            Ok(_) => panic!("wrong ACK!"),
            Err(_) => panic!("Timeout waiting for explorer"),
        }

    }

    #[test]
    fn test_complex_resource_generation() {
        //creating all channel
        let (orchestrator, explorer, snd_p_to_e) = setup_test();
        let forge=get_forge();
        //verify correct response to startAI
        orchestrator.orchestrator_to_planet
            .send(OrchestratorToPlanet::StartPlanetAI {})
            .unwrap();

        orchestrator.orchestrator_to_planet.send(OrchestratorToPlanet::IncomingExplorerRequest { explorer_id: 100, new_mpsc_sender: snd_p_to_e }).unwrap();
        orchestrator.orchestrator_to_planet.send(OrchestratorToPlanet::Sunray { 0: forge.generate_sunray() }).unwrap();
        explorer.explorer_to_planet.send(ExplorerToPlanet::GenerateResourceRequest { explorer_id: 100, resource: BasicResourceType::Oxygen }).unwrap();
        let mut generator=Generator::new();
        let mut cell=get_charged_cell();
        generator.add(BasicResourceType::Oxygen);
        generator.add(BasicResourceType::Hydrogen);
        let mut hydrogen=generator.make_hydrogen(&mut cell).unwrap();
        let mut cell=get_charged_cell();
        let mut oxygen=generator.make_oxygen(&mut cell).unwrap();
        let msg=ComplexResourceRequest::Water(hydrogen,oxygen);
        explorer.explorer_to_planet.send(ExplorerToPlanet::CombineResourceRequest { explorer_id: 100,msg  }).unwrap();

        match explorer.planet_to_explorer.recv_timeout(Duration::from_millis(300)) {
            Ok(_) => panic!("wrong ACK!"),
            Err(_) => assert!(true),
        }
    }
}



