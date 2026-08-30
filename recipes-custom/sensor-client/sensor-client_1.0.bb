SUMMARY = "Sensor Client Application To Gather Data From Sensors"
LICENSE = "CLOSED"


SRC_URI = " \
    file://client.c \
    file://client.h \
    file://process.c \
    file://process.h \
    file://Makefile \
"


S = "${WORKDIR}"


EXTRA_OEMAKE = "'CC=${CC}' 'CFLAGS=${CFLAGS}' 'LDFLAGS=${LDFLAGS}'"

do_compile() {
    oe_runmake
}

S = "${WORKDIR}"
do_install() {
    bbplain "S is: ${S}"
    bbplain "D is: ${D}"
    bbplain "bindir is: ${bindir}"
    install -d ${D}${bindir}
    install -m 0755 ${S}/sensor_client ${D}${bindir}/
}
