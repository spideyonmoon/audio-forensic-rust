"""Export separate pinned tool statistics v1; no native measurement mutation."""
import argparse
import json
import re
from generate_report_schema import ROOT, build_schema


def schema():
    source = "\n".join((ROOT/name).read_text(encoding="utf-8") for name in ["src/model.rs", "src/byproducts.rs", "src/tool_statistics.rs"])
    definitions=build_schema(source)["$defs"]; used={}
    def include(name):
        if name in used: return
        used[name]=definitions[name]
        for child in re.findall(r'"\$ref": "#/\$defs/(\w+)"', json.dumps(definitions[name])): include(child)
    include("ToolStatisticsReport")
    props=used["ToolStatisticsReport"]["properties"]
    props["statistics_version"]={"const":1}; props["contract_version"]={"const":1}
    props["method_id"]={"const":"ffmpeg-7.1.1-sox-14.4.2-v1"}
    used["ToolStatisticsReport"]["allOf"]=[dict(
        **{"if":{"properties":{"status":{"const":"analyzed"}}}},
        then={"properties":{key:{"$ref":"#/$defs/"+name} for key,name in [("stream","StreamInfo"),("coverage","Coverage"),("measurements","ToolMeasurements")]}},
        **{"else":{"properties":{key:{"type":"null"} for key in ["coverage","measurements"]}}})]
    used["ToolValue"]["allOf"]=[dict(
        **{"if":{"properties":{"availability":{"const":"available"}}}},
        then={"properties":{"value":{"type":"number"},"display":{"type":"string"},"reason":{"type":"null"}}},
        **{"else":{"properties":{"value":{"type":"null"},"display":{"type":"null"},"reason":{"type":"string"}}}})]
    for name in ["AstatsReport", "DrReport"]: used[name]["properties"]["channels"].update(minItems=1,maxItems=2)
    keys=["samplesRead","lengthSeconds","scaledBy","maximumAmplitude","minimumAmplitude","midlineAmplitude","meanNorm","meanAmplitude","rmsAmplitude","maximumDelta","minimumDelta","meanDelta","rmsDelta","roughFrequency","volumeAdjustment"]
    used["SoxReport"]["properties"]["fields"].update(required=keys, minProperties=15, maxProperties=15)
    common=["dc_offset","peak_dbfs","rms_dbfs","rms_peak_dbfs","rms_trough_dbfs","flat_factor_db","peak_count","noise_floor_dbfs","entropy","absolute_peak_count"]
    channel=common+["crest_linear","crest_db","amplitude_range_db","zero_crossings_rate"]
    used["AstatsReport"]["properties"]["overall"].update(required=common,minProperties=len(common),maxProperties=len(common))
    used["AstatsChannel"]["properties"]["fields"].update(required=channel,minProperties=len(channel),maxProperties=len(channel))
    audit=["peak_db","rms_db","rms_peak_db","rms_trough_db","noise_floor_db","dynamic_range_db","crest_factor_db","flat_factor","peak_count","sox_entropy","dc_offset","zero_crossings_rate"]
    used["ToolMeasurements"]["properties"]["legacy_loudness_profile"].update(required=audit,minProperties=len(audit),maxProperties=len(audit))
    return {"$schema":"https://json-schema.org/draft/2020-12/schema","$id":"urn:audio-forensic:tool-statistics:1","$ref":"#/$defs/ToolStatisticsReport","$defs":used}


def main():
    p=argparse.ArgumentParser(description=__doc__); p.add_argument("--check",action="store_true"); a=p.parse_args()
    artifact=schema(); data=json.dumps(artifact,indent=2,allow_nan=False)+"\n"; path=ROOT/"schemas/tool-statistics-1.schema.json"
    if a.check: assert path.read_text(encoding="utf-8")==data, "tool schema drift"
    else:
        with path.open("x",encoding="utf-8") as f: f.write(data)
    print(f"PASS: tool-statistics-1.schema.json, {len(artifact['$defs'])} definitions")


if __name__=="__main__": main()
