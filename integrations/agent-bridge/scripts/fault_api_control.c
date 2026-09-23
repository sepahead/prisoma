/* Synthetic API substitutions. Every signaling call uses the substitute below. */
#define proc_pidinfo controlled_proc_pidinfo
#define task_name_for_pid controlled_task_name_for_pid
#define task_info controlled_task_info
#define mach_port_deallocate controlled_mach_port_deallocate
#define proc_pidpath_audittoken controlled_proc_pidpath_audittoken
#define proc_signal_with_audittoken controlled_proc_signal_with_audittoken
#define getuid controlled_getuid
#define main observed_main
#include "darwin_fault.c"
#undef main

static const char *control;
static const char *expected_path = "/controlled/renderer";
static int observations, names, infos, releases, paths, signals, requested_signal;
static bool is(const char *value) { return strcmp(control, value) == 0; }
uid_t controlled_getuid(void) { return 501; }

int controlled_proc_pidinfo(int pid, int flavor, uint64_t arg, void *buffer, int size) {
    if (pid != 777 || flavor != PROC_PIDTBSDINFO || arg != 1
            || size != sizeof(struct proc_bsdinfo)) exit(90);
    observations++;
    if (is("unknown_observation")) { errno = 0; return 0; }
    if (is("missing_observation")) { errno = ESRCH; return 0; }
    if (is("short_observation")) return size - 1;
    struct proc_bsdinfo *row = buffer;
    memset(row, 0, sizeof(*row));
    row->pbi_pid = is("wrong_pid") ? 778 : 777;
    row->pbi_ppid = is("wrong_parent") || (is("parent_drift") && observations == 2) ? 701 : 700;
    row->pbi_uid = is("wrong_uid") || (is("uid_drift") && observations == 2) ? 502 : 501;
    row->pbi_ruid = is("wrong_ruid") || (is("ruid_drift") && observations == 2) ? 502 : 501;
    row->pbi_status = is("zero_status") ? 0 :
        is("zombie") || (is("becomes_zombie") && observations == 2) ? 5 : 2;
    row->pbi_start_tvsec = is("wrong_birth") || (is("birth_drift") && observations == 2)
        ? 1790000001 : 1790000000;
    row->pbi_start_tvusec = is("wrong_micros") || (is("micros_drift") && observations == 2) ? 2 : 1;
    return size;
}

kern_return_t controlled_task_name_for_pid(mach_port_name_t task, int pid, mach_port_name_t *name) {
    (void)task;
    if (pid != 777) exit(91);
    names++;
    if (is("task_name_failure")) return KERN_FAILURE;
    *name = is("null_task_name") ? MACH_PORT_NULL : is("dead_task_name") ? MACH_PORT_DEAD : 42;
    return KERN_SUCCESS;
}

kern_return_t controlled_task_info(task_name_t task, task_flavor_t flavor,
        task_info_t output, mach_msg_type_number_t *count) {
    (void)task;
    if (flavor != TASK_AUDIT_TOKEN || *count != TASK_AUDIT_TOKEN_COUNT) exit(92);
    infos++;
    if (is("task_info_failure")) return KERN_FAILURE;
    if (is("short_token")) *count -= 1;
    if (is("long_token")) *count += 1;
    audit_token_t *token = (audit_token_t *)output;
    memset(token, 0, sizeof(*token));
    token->val[1] = is("token_uid") ? 502 : 501;
    token->val[3] = is("token_ruid") ? 502 : 501;
    token->val[5] = is("token_pid") ? 778 : 777;
    token->val[7] = 12;
    return KERN_SUCCESS;
}

kern_return_t controlled_mach_port_deallocate(ipc_space_t task, mach_port_name_t name) {
    (void)task;
    (void)name;
    releases++;
    return is("release_failure") ? KERN_INVALID_NAME : KERN_SUCCESS;
}

int controlled_proc_pidpath_audittoken(audit_token_t *token, void *buffer, uint32_t size) {
    if (token->val[5] != 777 || token->val[7] != 12 || size != PROC_PIDPATHINFO_MAXSIZE) exit(93);
    paths++;
    if (is("path_failure")) return 0;
    if (is("path_negative")) return -1;
    if (is("path_full")) return size;
    if (is("path_unterminated")) { memset(buffer, 'x', size); return size - 1; }
    const char *path = is("path_wrong") ? "/other/executable" : expected_path;
    size_t length = strlen(path);
    memcpy(buffer, path, length + 1);
    if (is("path_token_mutation")) token->val[5] = 778;
    return (int)length + (is("path_length_mismatch") ? 1 : 0);
}

int controlled_proc_signal_with_audittoken(audit_token_t *token, int number) {
    signals++;
    requested_signal = number;
    if (token->val[5] != 777 || token->val[7] != 12 || number != SIGTERM
            || observations != 2 || paths != 1) exit(94);
    return is("kernel_version_reject") ? ESRCH : 0;
}

int main(int argc, char **argv) {
    if (argc != 2) return 95;
    control = argv[1];
    char long_path[PROC_PIDPATHINFO_MAXSIZE + 1];
    memset(long_path, 'x', sizeof(long_path));
    long_path[0] = '/';
    long_path[is("argument_path_limit") ? sizeof(long_path) - 2 : sizeof(long_path) - 1] = '\0';
    if (is("argument_path_limit")) expected_path = long_path;
    char *args[] = {"fault", "terminate", "777", "1790000000", "1", "700",
        (char *)expected_path, NULL};
    if (is("argument_wrong_command")) args[1] = "kill";
    if (is("argument_negative_pid")) args[2] = "-1";
    if (is("argument_pid_overflow")) args[2] = "2147483648";
    if (is("argument_signed_pid")) args[2] = "+777";
    if (is("argument_space_pid")) args[2] = " 777";
    if (is("argument_hex_pid")) args[2] = "0x309";
    if (is("argument_empty_birth")) args[3] = "";
    if (is("argument_zero_birth")) args[3] = "0";
    if (is("argument_birth_overflow")) args[3] = "18446744073709551616";
    if (is("argument_bad_micros")) args[4] = "1000000";
    if (is("argument_zero_parent")) args[5] = "0";
    if (is("argument_relative_path")) args[6] = "relative";
    if (is("argument_long_path")) args[6] = long_path;
    int result = observed_main(is("argument_count") ? 6 : 7, args);
    fprintf(stderr, "{\"observations\":%d,\"names\":%d,\"infos\":%d,"
        "\"releases\":%d,\"paths\":%d,\"signals\":%d,\"requested_signal\":%d}\n",
        observations, names, infos, releases, paths, signals, requested_signal);
    if (fflush(stderr) != 0 || ferror(stderr)) return 96;
    return result;
}
