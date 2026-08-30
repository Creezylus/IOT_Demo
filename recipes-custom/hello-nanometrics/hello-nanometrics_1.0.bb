SUMMARY = "Hello Nanometrics Application"
LICENSE = "CLOSED"

SRC_URI = "file://hello_nanometrics.c"

# Local files are unpacked directly into WORKDIR
S = "${WORKDIR}"

do_compile() {
    bbplain "I am compiling"
    ${CC} ${CFLAGS} ${LDFLAGS} hello_nanometrics.c -o hello-nanometrics
}

do_install() {
    bbplain "S is: ${S}"
    bbplain "D is: ${D}"
    bbplain "bindir is: ${bindir}"
    install -d ${D}${bindir}
    install -m 0755 ${S}/hello-nanometrics ${D}${bindir}/
}
