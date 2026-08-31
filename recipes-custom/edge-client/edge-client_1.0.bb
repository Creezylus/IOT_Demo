SUMMARY = "Edge Client Application To Gather Data From Multiple Sensor Clients"
LICENSE = "CLOSED"


SRC_URI = " \
    file://server.c \
    file://server.h \
    file://log.c \
    file://log.h \
    file://Makefile \
"


S = "${WORKDIR}"

EXTRA_OEMAKE = "'CC=${CC}' 'CFLAGS=${CFLAGS}' 'LDFLAGS=${LDFLAGS}'"

do_compile() {
    oe_runmake
}
do_install() {
    bbplain "S is: ${S}"
    bbplain "D is: ${D}"
    bbplain "bindir is: ${bindir}"
    install -d ${D}${bindir}
    install -m 0755 ${S}/edge_client ${D}${bindir}/
}
