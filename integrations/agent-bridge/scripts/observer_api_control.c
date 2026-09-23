/* Synthetic libproc substitutions; retain the actual installed SDK declarations. */
#define proc_listallpids controlled_proc_listallpids
#define proc_pidinfo controlled_proc_pidinfo
#define proc_pidpath controlled_proc_pidpath
#define main observed_main
#include "darwin_observer.c"
#undef main

static const char *control;
static int calls;
static int is(const char *value) { return strcmp(control, value) == 0; }

int controlled_proc_listallpids(void *buffer, int bytes) {
    if (bytes != MAX_PIDS * sizeof(int)) exit(90);
    if (is("negative_roster")) return -1;
    if (is("zero_roster")) return 0;
    if (is("full_roster")) return MAX_PIDS;
    int count = is("max_admitted_roster") ? MAX_PIDS - 1 :
        is("duplicate_pid") || is("negative_after_valid") ? 2 : 1;
    for (int i = 0; i < count; ++i) {
        ((int *)buffer)[i] = is("negative_pid") || (is("negative_after_valid") && i) ? -3 :
            is("zero_pid") ? 0 : is("duplicate_pid") ? 777 : i + 1;
    }
    return count;
}

int controlled_proc_pidinfo(int pid, int flavor, uint64_t arg, void *buffer, int size) {
    if (flavor != PROC_PIDTBSDINFO || arg != 1 || size != sizeof(struct proc_bsdinfo)) exit(91);
    calls++;
    if (is("unknown_zero")) { errno = 0; return 0; }
    if (is("missing")) { errno = ESRCH; return 0; }
    if (is("unavailable")) { errno = EPERM; return 0; }
    if (is("short_info")) return size - 1;
    struct proc_bsdinfo *info = buffer;
    memset(info, 0, sizeof(*info));
    info->pbi_pid = is("pid_mismatch") ? (uint32_t)(pid + 1) : (uint32_t)pid;
    info->pbi_ppid = is("parent_drift") && calls > 1 ? 21 : 20;
    info->pbi_uid = is("uid_drift") && calls > 1 ? 502 : 501;
    info->pbi_status = 2;
    info->pbi_start_tvsec = is("zero_birth") ? 0 :
        is("birth_drift") && calls > 1 ? 1790000001 : 1790000000;
    info->pbi_start_tvusec = is("bad_microseconds") ? 1000000 : 1;
    return size;
}

int controlled_proc_pidpath(int pid, void *buffer, uint32_t size) {
    (void)pid;
    if (is("full_path")) return size;
    if (is("empty_path")) return 1;
    const char path[] = "/observed/path";
    memcpy(buffer, path, sizeof(path));
    return sizeof(path) - 1;
}

int main(int argc, char **argv) {
    if (argc != 3) return 92;
    control = argv[1];
    char *args[] = {"observer", argv[2], "777", NULL};
    return observed_main(strcmp(argv[2], "selected") == 0 ? 3 : 2, args);
}
