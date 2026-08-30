SUMMARY = "Sensor Client Application"
LICENSE = "CLOSED"

# Explicitly fetch every file needed for the build
SRC_URI = " \
    file://client.c \
    file://client.h \
    file://process.c \
    file://process.h \
    file://Makefile \
"

# Point the build directory to where the files are actually unpacked
S = "${WORKDIR}"

# Pass BitBake's environment variables to your Makefile safely
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
