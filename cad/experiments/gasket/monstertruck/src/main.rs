use monstertruck::modeling::*;
use monstertruck::meshing::prelude::*;
use monstertruck::solid::difference;
use monstertruck::step::save::{CompleteStepDisplay, StepModel};
use serde_json::{Value, json};
use std::time::Instant;

fn n(v: &Value, key: &str) -> f64 { v[key].as_f64().unwrap() }
fn polygon(points: &Value, z: f64, height: f64) -> Solid {
    let mut points: Vec<_> = points.as_array().unwrap().iter().map(|p| (n(p,"x"),n(p,"y"))).collect();
    let area: f64 = (0..points.len()).map(|i| { let a=points[i];let b=points[(i+1)%points.len()];a.0*b.1-b.0*a.1 }).sum();
    if area<0. { points.reverse(); }
    let vertices=builder::vertices(points.iter().map(|p| Point3::new(p.0,p.1,z)));
    let wire: Wire=(0..vertices.len()).map(|i| builder::line(&vertices[i],&vertices[(i+1)%vertices.len()])).collect();
    let face: Face=builder::try_attach_plane(vec![wire]).unwrap();
    builder::extrude(&face,Vector3::new(0.,0.,height))
}
fn cylinder(x: f64,y: f64,z: f64,r: f64,height: f64) -> Solid {
    let a=builder::vertex(Point3::new(x+r,y,z));let b=builder::vertex(Point3::new(x-r,y,z));
    let wire: Wire=vec![builder::circle_arc(&a,&b,Point3::new(x,y+r,z)),
        builder::circle_arc(&b,&a,Point3::new(x,y-r,z))].into();
    let face: Face=builder::try_attach_plane(vec![wire]).unwrap();
    builder::extrude(&face,Vector3::new(0.,0.,height))
}
fn main() -> std::result::Result<(),Box<dyn std::error::Error>> {
    let args: Vec<_>=std::env::args().collect();
    let input: Value=serde_json::from_str(&std::fs::read_to_string(&args[1])?)?;
    let output=std::path::Path::new(&args[2]);std::fs::create_dir_all(output)?;
    let body=&input["body"];let z=n(body,"z");let height=n(body,"thickness");
    assert_eq!(body["kind"],"plate");
    let start=Instant::now();let mut solids=Vec::new();
    let overrun=std::env::var("MONSTERTRUCK_CUTTER_OVERRUN").ok().map(|x| x.parse::<f64>().unwrap()).unwrap_or(0.);
    let tolerance=std::env::var("MONSTERTRUCK_BOOLEAN_TOLERANCE").ok().map(|x| x.parse::<f64>().unwrap()).unwrap_or(1e-7);
    for (ri,region) in input["regions"].as_array().unwrap().iter().enumerate() {
        assert!(region["holes"].as_array().unwrap().is_empty());
        let mut solid=polygon(&region["outer"],z,height);
        eprintln!("region {ri} base {:?}",start.elapsed());
        for (mi,mount) in region["mounts"].as_array().unwrap().iter().enumerate() {
            assert_eq!(mount["kind"],"hole");
            let tool=cylinder(n(&mount["at"],"x"),n(&mount["at"],"y"),z-overrun,n(mount,"holeDiameter")/2.,height+2.*overrun);
            if ri==0 && mi==0 && std::env::var_os("MONSTERTRUCK_DIAGNOSTIC_STEP").is_some() {
                for (name,shape) in [("base",&solid),("cutter",&tool)] {
                    let compressed=shape.compress();
                    std::fs::write(output.join(format!("{name}.step")),CompleteStepDisplay::new(StepModel::from(&compressed),Default::default()).to_string())?;
                }
            }
            solid=difference(&solid,&tool,tolerance).map_err(|e| format!("region {ri} mount {mi}: {e:?}"))?;
            eprintln!("region {ri} mount {mi} {:?}",start.elapsed());
        }
        for (oi,opening) in body["openings"].as_array().unwrap().iter().enumerate() {
            let tool=polygon(&opening["points"],n(opening,"z"),n(opening,"height"));
            solid=difference(&solid,&tool,tolerance).map_err(|e| format!("region {ri} opening {oi}: {e:?}"))?;
            eprintln!("region {ri} opening {oi} {:?}",start.elapsed());
        }
        solids.push(solid);
    }
    let built=start.elapsed().as_secs_f64()*1000.;
    let mut positions=Vec::<f32>::new();let mut normals=Vec::<f32>::new();
    for solid in &solids {
        let mesh=solid.triangulation(0.1).to_polygon();
        for face in mesh.face_iter() {
            assert_eq!(face.len(),3);
            for v in face { let p=mesh.positions()[v.pos];let normal=mesh.normals()[v.nor.unwrap()];
                positions.extend([p.x as f32,p.y as f32,p.z as f32]);
                normals.extend([normal.x as f32,normal.y as f32,normal.z as f32]); }
        }
    }
    let total=start.elapsed().as_secs_f64()*1000.;
    for (i,solid) in solids.iter().enumerate() {
        let compressed=solid.compress();
        std::fs::write(output.join(format!("region-{i}.step")),CompleteStepDisplay::new(StepModel::from(&compressed),Default::default()).to_string())?;
    }
    let report=json!({"engine":"monstertruck","version":"0.4.1","target":"native","constructionMs":built,
        "generationMs":total,"meshMs":total-built,"triangles":positions.len()/9,
        "meshBytes":(positions.len()+normals.len())*4,"solids":solids.len()});
    std::fs::write(output.join("metrics.json"),serde_json::to_string_pretty(&report)?)?;
    std::fs::write(output.join("mesh.json"),serde_json::to_vec(&json!({"positions":positions,"normals":normals}))?)?;
    println!("{report}");Ok(())
}
