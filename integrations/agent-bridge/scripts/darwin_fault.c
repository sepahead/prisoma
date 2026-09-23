/* A fixed SIGTERM fault for an externally selected, observed process identity.
 * The supervisor establishes ownership. This helper rejoins birth, parent, UID,
 * and executable, then asks the kernel to compare the audit-token PID version.
 * It does not establish descendant ownership or hostile-process containment.
 */
#include <errno.h>
#include <inttypes.h>
#include <libproc.h>
#include <limits.h>
#include <mach/mach.h>
#include <mach/mach_traps.h>
#include <signal.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/proc_info.h>
#include <unistd.h>

static bool number(const char *text, uint64_t lower, uint64_t upper, uint64_t *out) {
    if (!text[0]) return false;
    for (const char *p = text; *p; ++p) if (*p < '0' || *p > '9') return false;
    errno = 0;
    char *end = NULL;
    unsigned long long value = strtoull(text, &end, 10);
    if (errno || !end || *end || value < lower || value > upper) return false;
    *out = value;
    return true;
}

static bool matches(int pid, uint64_t seconds, uint64_t micros, uint64_t parent) {
    struct proc_bsdinfo info = {0};
    int size = proc_pidinfo(pid, PROC_PIDTBSDINFO, 1, &info, sizeof(info));
    return size == sizeof(info) && info.pbi_pid == (uint32_t)pid
        && info.pbi_ppid == parent && info.pbi_uid == getuid()
        && info.pbi_ruid == getuid() && info.pbi_status > 0 && info.pbi_status < 5
        && info.pbi_start_tvsec == seconds && info.pbi_start_tvusec == micros;
}

int main(int argc, char **argv) {
    uint64_t pid, seconds, micros, parent;
    if (argc != 7 || strcmp(argv[1], "terminate") != 0
            || !number(argv[2], 1, INT_MAX, &pid)
            || !number(argv[3], 1, UINT64_MAX, &seconds)
            || !number(argv[4], 0, 999999, &micros)
            || !number(argv[5], 1, INT_MAX, &parent)
            || argv[6][0] != '/' || strlen(argv[6]) >= PROC_PIDPATHINFO_MAXSIZE)
        return 10;
    if (!matches((int)pid, seconds, micros, parent)) return 20;
    mach_port_t name = MACH_PORT_NULL;
    if (task_name_for_pid(mach_task_self(), (int)pid, &name) != KERN_SUCCESS
            || name == MACH_PORT_NULL || name == MACH_PORT_DEAD) return 21;
    audit_token_t token = {{0}};
    mach_msg_type_number_t count = TASK_AUDIT_TOKEN_COUNT;
    kern_return_t info = task_info(name, TASK_AUDIT_TOKEN, (task_info_t)&token, &count);
    kern_return_t released = mach_port_deallocate(mach_task_self(), name);
    if (released != KERN_SUCCESS || info != KERN_SUCCESS || count != TASK_AUDIT_TOKEN_COUNT
            || token.val[5] != pid || token.val[1] != getuid() || token.val[3] != getuid())
        return 22;
    char executable[PROC_PIDPATHINFO_MAXSIZE] = {0};
    audit_token_t path_token = token;
    int length = proc_pidpath_audittoken(&path_token, executable, sizeof(executable));
    if (length <= 0 || (size_t)length >= sizeof(executable)
            || memcmp(&path_token, &token, sizeof(token)) != 0
            || strnlen(executable, sizeof(executable)) != (size_t)length
            || strcmp(executable, argv[6]) != 0) return 23;
    if (!matches((int)pid, seconds, micros, parent)) return 24;
    int result = proc_signal_with_audittoken(&token, SIGTERM);
    printf("{\"schema\":\"local.darwin-version-bound-fault.v1\","
           "\"pid\":%" PRIu64 ",\"start_seconds\":%" PRIu64 ","
           "\"start_microseconds\":%" PRIu64 ",\"ppid\":%" PRIu64 ","
           "\"pid_version\":%u,\"signal\":%d,\"signal_result\":%d}\n",
           pid, seconds, micros, parent, token.val[7], SIGTERM, result);
    if (fflush(stdout) != 0 || ferror(stdout)) return 25;
    return result == 0 ? 0 : 26;
}
