pub(super) fn exchange(data: &str) -> String {
    format!(
        "ISO-10303-21;HEADER;FILE_DESCRIPTION(('unit resolver test'),'2;1');FILE_NAME('units.step','','',(),'','','');FILE_SCHEMA(('AUTOMOTIVE_DESIGN'));ENDSEC;DATA;{data}ENDSEC;END-ISO-10303-21;"
    )
}
