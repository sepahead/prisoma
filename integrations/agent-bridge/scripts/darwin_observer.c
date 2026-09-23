/* Read-only process observations using the installed Darwin SDK ABI.
 * No signal, wait, ownership, or process-creation operation is exposed here.
 */
#include <errno.h>
#include <inttypes.h>
#include <libproc.h>
#include <limits.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/proc_info.h>
#include <unistd.h>

#define MAX_PIDS 32768

static int observe(int pid, struct proc_bsdinfo *info) {
    memset(info, 0, sizeof(*info));
    errno = 0;
    /* Darwin's nonzero arg includes an unreaped zombie in this observation. */
    int count = proc_pidinfo(pid, PROC_PIDTBSDINFO, 1, info, sizeof(*info));
    if (count == 0 && errno == ESRCH) return 0;
    if (count == 0 && (errno == EPERM || errno == EACCES)) return -2;
    if (count != sizeof(*info) || info->pbi_pid != (uint32_t)pid ||
        info->pbi_start_tvsec == 0 || info->pbi_start_tvusec >= 1000000) return -1;
    return 1;
}

static void row(const struct proc_bsdinfo *info) {
    printf("{\"pid\":%u,\"ppid\":%u,\"uid\":%u,\"status\":%u,"
           "\"start_seconds\":%" PRIu64 ",\"start_microseconds\":%" PRIu64 "}",
           info->pbi_pid, info->pbi_ppid, info->pbi_uid, info->pbi_status,
           info->pbi_start_tvsec, info->pbi_start_tvusec);
}

static int snapshot(void) {
    int *pids = calloc(MAX_PIDS, sizeof(int));
    int *unavailable = calloc(MAX_PIDS, sizeof(int));
    if (!pids || !unavailable) { free(pids); free(unavailable); return 2; }
    int count = proc_listallpids(pids, MAX_PIDS * sizeof(int));
    if (count <= 0 || count >= MAX_PIDS) { free(pids); free(unavailable); return 3; }
    printf("{\"schema\":\"local.darwin-process-observation.v1\",\"bsdinfo_bytes\":%zu,\"rows\":[", sizeof(struct proc_bsdinfo));
    bool first = true;
    int missing = 0;
    for (int index = 0; index < count; ++index) {
        if (pids[index] < 0) { free(pids); free(unavailable); return 4; }
        if (pids[index] == 0) continue;
        struct proc_bsdinfo info;
        int state = observe(pids[index], &info);
        if (state == -2) { unavailable[missing++] = pids[index]; continue; }
        if (state < 0) { free(pids); free(unavailable); return 4; }
        if (state == 0) continue;
        if (!first) putchar(',');
        row(&info);
        first = false;
    }
    printf("],\"unavailable\":[");
    for (int index = 0; index < missing; ++index) printf("%s%d", index ? "," : "", unavailable[index]);
    printf("]}\n");
    free(pids);
    free(unavailable);
    return fflush(stdout) != 0 || ferror(stdout) ? 5 : 0;
}

static int selected(int pid) {
    struct proc_bsdinfo before, after;
    if (observe(pid, &before) != 1) return 6;
    char path[PROC_PIDPATHINFO_MAXSIZE] = {0};
    int bytes = proc_pidpath(pid, path, sizeof(path));
    if (bytes <= 0 || (size_t)bytes >= sizeof(path) || observe(pid, &after) != 1) return 7;
    if (before.pbi_start_tvsec != after.pbi_start_tvsec ||
        before.pbi_start_tvusec != after.pbi_start_tvusec ||
        before.pbi_ppid != after.pbi_ppid || before.pbi_uid != after.pbi_uid) return 8;
    size_t length = strnlen(path, sizeof(path));
    if (!length || length == sizeof(path)) return 9;
    printf("{\"identity\":");
    row(&after);
    printf(",\"executable_path_hex\":\"");
    for (size_t index = 0; index < length; ++index) printf("%02x", (unsigned char)path[index]);
    printf("\"}\n");
    return fflush(stdout) != 0 || ferror(stdout) ? 5 : 0;
}

int main(int argc, char **argv) {
    if (argc == 2 && strcmp(argv[1], "snapshot") == 0) return snapshot();
    if (argc == 3 && strcmp(argv[1], "selected") == 0) {
        char *end = NULL;
        errno = 0;
        long pid = strtol(argv[2], &end, 10);
        if (errno || end == argv[2] || *end || pid <= 0 || pid > INT_MAX) return 10;
        return selected((int)pid);
    }
    fputs("usage: observer snapshot | observer selected POSITIVE_PID\n", stderr);
    return 10;
}
