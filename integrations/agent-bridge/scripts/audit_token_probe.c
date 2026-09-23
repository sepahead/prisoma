#include <errno.h>
#include <libproc.h>
#include <mach/mach.h>
#include <mach/mach_traps.h>
#include <signal.h>
#include <stdio.h>
#include <sys/wait.h>
#include <unistd.h>

int main(void) {
    int channel[2];
    if (pipe(channel) != 0) return 2;
    pid_t child = fork();
    if (child < 0) return 2;
    if (child == 0) {
        close(channel[1]);
        char byte;
        while (read(channel[0], &byte, 1) < 0 && errno == EINTR) {}
        _exit(0);
    }
    close(channel[0]);
    mach_port_t name = MACH_PORT_NULL;
    kern_return_t named = task_name_for_pid(mach_task_self(), child, &name);
    audit_token_t token = {{0}};
    mach_msg_type_number_t count = TASK_AUDIT_TOKEN_COUNT;
    kern_return_t info = named == KERN_SUCCESS
        ? task_info(name, TASK_AUDIT_TOKEN, (task_info_t)&token, &count)
        : named;
    int wrong_result = -1, signal_result = -1, premature = -1;
    if (info == KERN_SUCCESS && count == TASK_AUDIT_TOKEN_COUNT
            && token.val[5] == (unsigned)child) {
        audit_token_t wrong = token;
        wrong.val[7] ^= 0x80000000U;
        wrong_result = proc_signal_with_audittoken(&wrong, SIGTERM);
        int interim = 0;
        premature = waitpid(child, &interim, WNOHANG);
        if (wrong_result == ESRCH && premature == 0)
            signal_result = proc_signal_with_audittoken(&token, SIGTERM);
    }
    if (name != MACH_PORT_NULL) mach_port_deallocate(mach_task_self(), name);
    close(channel[1]);
    int status = 0;
    pid_t waited = waitpid(child, &status, 0);
    printf("{\"task_name_result\":%d,\"task_info_result\":%d,\"count\":%u,"
           "\"token_pid_matches\":%s,\"wrong_version_signal_result\":%d,"
           "\"child_survived_wrong_version\":%s,\"signal_result\":%d,"
           "\"waited\":%s,\"terminated_by_sigterm\":%s}\n",
           named, info, count, token.val[5] == (unsigned)child ? "true" : "false",
           wrong_result, premature == 0 ? "true" : "false", signal_result,
           waited == child ? "true" : "false",
           waited == child && WIFSIGNALED(status) && WTERMSIG(status) == SIGTERM
               ? "true" : "false");
    if (fflush(stdout) != 0 || ferror(stdout)) return 3;
    return info == KERN_SUCCESS && wrong_result == ESRCH && premature == 0
        && signal_result == 0 && waited == child && WIFSIGNALED(status)
        && WTERMSIG(status) == SIGTERM ? 0 : 1;
}
