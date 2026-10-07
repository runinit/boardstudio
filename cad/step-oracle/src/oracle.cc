// Narrow OCCT bridge: read a STEP file with STEPControl_Reader (no sewing, no healing beyond
// what the reader does itself) and report facts about the resulting shape.
#include "boardstudio-step-oracle/src/main.rs.h"

#include <BRepBndLib.hxx>
#include <BRepCheck_Analyzer.hxx>
#include <BRepGProp.hxx>
#include <Bnd_Box.hxx>
#include <GProp_GProps.hxx>
#include <IFSelect_ReturnStatus.hxx>
#include <Message.hxx>
#include <Message_Messenger.hxx>
#include <Message_PrinterOStream.hxx>
#include <STEPControl_Reader.hxx>
#include <Standard_Failure.hxx>
#include <TopAbs_ShapeEnum.hxx>
#include <TopExp.hxx>
#include <TopExp_Explorer.hxx>
#include <TopTools_IndexedMapOfShape.hxx>
#include <TopoDS_Shape.hxx>

#include <string>

namespace bs_oracle {

Facts inspect_file(rust::Str path) {
  Facts facts{};
  facts.read_status = -1;
  try {
    // OCCT prints reader chatter to stdout; the CLI reserves stdout for its report.
    Message::DefaultMessenger()->RemovePrinters(STANDARD_TYPE(Message_PrinterOStream));

    const std::string file(path);
    STEPControl_Reader reader;
    const IFSelect_ReturnStatus status = reader.ReadFile(file.c_str());
    facts.read_status = static_cast<int32_t>(status);
    if (status != IFSelect_RetDone) {
      facts.error = rust::String("STEPControl_Reader::ReadFile did not return RetDone");
      return facts;
    }
    reader.TransferRoots();
    const TopoDS_Shape shape = reader.OneShape();
    if (shape.IsNull()) {
      facts.valid = false;
      facts.error = rust::String("STEP transfer produced no shape");
      return facts;
    }

    TopTools_IndexedMapOfShape allFaces;
    TopTools_IndexedMapOfShape solidFaces;
    TopExp::MapShapes(shape, TopAbs_FACE, allFaces);
    for (TopExp_Explorer solids(shape, TopAbs_SOLID); solids.More(); solids.Next()) {
      facts.solid_count += 1;
      TopExp::MapShapes(solids.Current(), TopAbs_FACE, solidFaces);
      for (TopExp_Explorer shells(solids.Current(), TopAbs_SHELL); shells.More(); shells.Next()) {
        facts.shell_count += 1;
      }
    }
    facts.face_count = static_cast<uint32_t>(allFaces.Extent());
    facts.orphan_faces = static_cast<uint32_t>(allFaces.Extent() - solidFaces.Extent());

    const BRepCheck_Analyzer analyzer(shape, true, false, true);
    facts.valid = analyzer.IsValid();

    // Same measurement settings the retired libcascade harness used.
    GProp_GProps props;
    BRepGProp::VolumeProperties(shape, props, true, true, false);
    facts.volume = props.Mass();
    Bnd_Box box;
    BRepBndLib::AddOptimal(shape, box, false, false);
    if (!box.IsVoid()) {
      facts.min_x = box.CornerMin().X();
      facts.min_y = box.CornerMin().Y();
      facts.min_z = box.CornerMin().Z();
      facts.max_x = box.CornerMax().X();
      facts.max_y = box.CornerMax().Y();
      facts.max_z = box.CornerMax().Z();
    }
  } catch (const Standard_Failure& failure) {
    facts.valid = false;
    facts.error = rust::String(std::string("OCCT exception: ") + failure.what());
  } catch (const std::exception& failure) {
    facts.valid = false;
    facts.error = rust::String(std::string("C++ exception: ") + failure.what());
  }
  return facts;
}

}  // namespace bs_oracle
