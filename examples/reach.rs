/*
Copyright 2017 Takashi Ogura

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
*/
extern crate env_logger;
extern crate gear;
extern crate glfw;
extern crate k;
extern crate nalgebra as na;
extern crate ncollide;
extern crate rand;
extern crate structopt;
extern crate urdf_rs;
extern crate urdf_viz;

use glfw::{Action, Key, WindowEvent};
use std::path::Path;
use ncollide::shape::{Compound, Cuboid, ShapeHandle};
use k::JointContainer;


struct CollisionAvoidApp<'a> {
    viewer: urdf_viz::Viewer<'a>,
    target_objects: Compound<na::Point3<f64>, na::Isometry3<f64>>,
    ik_target_pose: na::Isometry3<f64>,
    colliding_link_names: Vec<String>,
    robot: k::LinkTree<f64>,
    planner: gear::CollisionAvoidJointPathPlanner<k::RefKinematicChain<f64>>,
}

impl<'a> CollisionAvoidApp<'a> {
    fn new(urdf_robot: &'a urdf_rs::Robot, base_dir: Option<&Path>) -> Self {
        let mut viewer = urdf_viz::Viewer::new(urdf_robot);

        viewer.setup(base_dir, false);
        let base_transform = na::Isometry3::from_parts(
            na::Translation3::new(0.0, 0.0, 0.0),
            na::UnitQuaternion::from_euler_angles(0.0, 1.57, 1.57),
        );

        let checker_for_planner = gear::CollisionChecker::<f64>::new(urdf_robot, base_dir, 0.01);
        let mut robot_for_planner = k::urdf::create_tree::<f64>(urdf_robot);
        robot_for_planner.set_root_transform(base_transform);
        viewer.update(&robot_for_planner);

        let mut arms = k::create_kinematic_chains_with_dof_limit(&robot_for_planner, 7);
        let planner = gear::CollisionAvoidJointPathPlannerBuilder::new(
            arms.pop().expect("no arm"),
            checker_for_planner,
        ).max_try(5000)
            .finalize();

        viewer.add_axis_cylinders("origin", 1.0);
        if let Some(obj) = viewer.scenes.get_mut("origin") {
            obj.0.set_local_transformation(na::convert(base_transform));
        }

        let target_shape1 = Cuboid::new(na::Vector3::new(0.20, 0.3, 0.1));
        let target_pose1 =
            base_transform * na::Isometry3::new(na::Vector3::new(0.6, 0.0, 0.1), na::zero());
        let mut cube = viewer.window.add_cube(
            target_shape1.half_extents()[0] as f32 * 2.0,
            target_shape1.half_extents()[1] as f32 * 2.0,
            target_shape1.half_extents()[2] as f32 * 2.0,
        );
        cube.set_local_transformation(na::convert(target_pose1));
        cube.set_color(0.5, 0.0, 0.5);

        let target_shape2 = Cuboid::new(na::Vector3::new(0.20, 0.3, 0.1));
        let target_pose2 =
            base_transform * na::Isometry3::new(na::Vector3::new(0.6, 0.0, 0.6), na::zero());
        let mut cube2 = viewer.window.add_cube(
            target_shape2.half_extents()[0] as f32 * 2.0,
            target_shape2.half_extents()[1] as f32 * 2.0,
            target_shape2.half_extents()[2] as f32 * 2.0,
        );
        cube2.set_local_transformation(na::convert(target_pose2));
        cube2.set_color(0.5, 0.5, 0.0);

        let mut shapes = Vec::new();
        let handle1 = ShapeHandle::new(target_shape1);
        shapes.push((target_pose1, handle1));
        let handle2 = ShapeHandle::new(target_shape2);
        shapes.push((target_pose2, handle2));
        let target_objects = Compound::new(shapes);

        let ik_target_pose = base_transform
            * na::Isometry3::from_parts(
                na::Translation3::new(0.60, 0.40, 0.3),
                na::UnitQuaternion::from_euler_angles(0.0, -0.1, 0.0),
            );
        viewer.add_axis_cylinders("ik_target", 0.3);
        CollisionAvoidApp {
            viewer: viewer,
            target_objects: target_objects,
            ik_target_pose: ik_target_pose,
            colliding_link_names: Vec::new(),
            planner: planner,
            robot: robot_for_planner,
        }
    }
    fn update_robot(&mut self) {
        self.viewer.update(&self.robot);
    }
    fn update_ik_target(&mut self) {
        if let Some(obj) = self.viewer.scenes.get_mut("ik_target") {
            obj.0
                .set_local_transformation(na::convert(self.ik_target_pose));
        }
    }
    fn reset_colliding_link_colors(&mut self) {
        for link in &self.colliding_link_names {
            self.viewer.reset_temporal_color(link);
        }
    }
    fn run(&mut self) {
        self.update_robot();
        self.update_ik_target();
        let mut plans: Vec<Vec<f64>> = Vec::new();
        let solver = k::JacobianIKSolverBuilder::<f64>::new()
            .num_max_try(1000)
            .allowable_target_distance(0.01)
            .move_epsilon(0.00001)
            .jacobian_move_epsilon(0.001)
            .finalize();
        let mut initial = self.planner.robot.get_joint_angles();
        while self.viewer.render() {
            if !plans.is_empty() {
                self.planner
                    .set_joint_angles(&plans.pop().unwrap())
                    .unwrap();
                self.update_robot();
            }

            for event in self.viewer.events().iter() {
                match event.value {
                    WindowEvent::Key(code, _, Action::Press, _) => match code {
                        Key::Up => {
                            self.ik_target_pose.translation.vector[1] += 0.05;
                            self.update_ik_target();
                        }
                        Key::Down => {
                            self.ik_target_pose.translation.vector[1] -= 0.05;
                            self.update_ik_target();
                        }
                        Key::Left => {
                            self.ik_target_pose.translation.vector[0] -= 0.05;
                            self.update_ik_target();
                        }
                        Key::Right => {
                            self.ik_target_pose.translation.vector[0] += 0.05;
                            self.update_ik_target();
                        }
                        Key::B => {
                            self.ik_target_pose.translation.vector[2] += 0.05;
                            self.update_ik_target();
                        }
                        Key::F => {
                            self.ik_target_pose.translation.vector[2] -= 0.05;
                            self.update_ik_target();
                        }
                        Key::I => {
                            self.reset_colliding_link_colors();
                            let result = gear::solve_ik_with_random_initialize(
                                &solver,
                                &mut self.planner.robot,
                                &self.ik_target_pose,
                                100,
                            );
                            if result.is_ok() {
                                self.update_robot();
                            } else {
                                println!("fail!!");
                            }
                        }
                        Key::M => {
                            initial = self.planner.robot.get_joint_angles();
                        }
                        Key::P => {
                            let goal = self.planner.robot.get_joint_angles();
                            self.planner.set_joint_angles(&initial).unwrap();
                            let result = self.planner.plan(&goal, &self.target_objects);
                            match result {
                                Ok(mut plan) => {
                                    plan.reverse();
                                    for i in 0..(plan.len() - 1) {
                                        let mut interpolated_angles =
                                            gear::interpolate(&plan[i], &plan[i + 1], 0.1);
                                        plans.append(&mut interpolated_angles);
                                    }
                                }
                                Err(err) => {
                                    println!("{:?}", err);
                                }
                            }
                        }
                        Key::R => {
                            self.reset_colliding_link_colors();
                            gear::set_random_joint_angles(&mut self.planner.robot).unwrap();
                            self.update_robot();
                        }
                        Key::C => {
                            self.colliding_link_names =
                                self.planner.get_colliding_link_names(&self.target_objects);
                            for name in &self.colliding_link_names {
                                println!("{}", name);
                                self.viewer.set_temporal_color(name, 0.8, 0.8, 0.6);
                            }
                            println!("===========");
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
        }
    }
}

fn main() {
    use std::env;
    env_logger::init().unwrap();
    let input_string = env::args().nth(1).unwrap_or("sample.urdf".to_owned());
    let input_path = Path::new(&input_string);
    let base_dir = input_path.parent();
    let urdf_robot = urdf_rs::utils::read_urdf_or_xacro(input_path).unwrap();
    let mut app = CollisionAvoidApp::new(&urdf_robot, base_dir);
    app.run();
}