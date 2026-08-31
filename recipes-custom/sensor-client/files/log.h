#ifndef IOTLOGGER_H
#define IOTLOGGER_H

#include <stdarg.h>

#define iotlogger(...) custom_log_impl(__VA_ARGS__)

void custom_log_impl(const char *format, ...);

#endif // IOTLOGGER_H
