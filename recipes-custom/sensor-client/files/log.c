#define _GNU_SOURCE 

#include "log.h"
#include <stdio.h>
#include <stdlib.h>
#include <stdarg.h>
#include <string.h>
#include <time.h>
#include <pthread.h>
#include <errno.h>

#if defined(__linux__) || defined(__GLIBC__)
    extern char *program_invocation_short_name;
    #define GET_PROG_NAME() program_invocation_short_name
#else
    #define GET_PROG_NAME() "unknown_process"
#endif

static FILE *log_file = NULL;
static pthread_mutex_t log_mutex = PTHREAD_MUTEX_INITIALIZER;

static void init_logging(void) {
    if (log_file != NULL) return;

    const char *prog_name = GET_PROG_NAME();
    if (!prog_name) prog_name = "unknown";

    char filepath[512];
    snprintf(filepath, sizeof(filepath), "/var/tmp/%s.log", prog_name);

    log_file = fopen(filepath, "a");
    if (!log_file) {
        fprintf(stderr, "Failed to open log file %s: %s\n", filepath, strerror(errno));
        log_file = stderr;
    }
}

static void __attribute__((destructor)) cleanup_logging(void) {
    if (log_file && log_file != stderr) {
        fclose(log_file);
        log_file = NULL;
    }
}

void custom_log_impl(const char *format, ...) {
    pthread_mutex_lock(&log_mutex);
    
    if (!log_file) {
        init_logging();
    }

    time_t now;
    time(&now);
    struct tm *tm_info = localtime(&now);
    char time_buf[26];
    strftime(time_buf, sizeof(time_buf), "%Y-%m-%d %H:%M:%S", tm_info);

    fprintf(log_file, "[%s] ", time_buf);
    
    va_list args;
    va_start(args, format);
    vfprintf(log_file, format, args);
    va_end(args);
    
    fflush(log_file); 
    
    pthread_mutex_unlock(&log_mutex);
}
