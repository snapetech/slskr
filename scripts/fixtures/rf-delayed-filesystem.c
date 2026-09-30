/* Isolated RF-047 fixture: passthrough storage with delayed event-file fsync. */
#define FUSE_USE_VERSION 31
#define _GNU_SOURCE
#include <fuse3/fuse.h>
#include <dirent.h>
#include <errno.h>
#include <fcntl.h>
#include <limits.h>
#include <stdio.h>
#include <string.h>
#include <sys/statvfs.h>
#include <time.h>
#include <unistd.h>

static const char *backing;
static const char *control;

static int physical(const char *path, char output[PATH_MAX]) {
    int length = snprintf(output, PATH_MAX, "%s%s", backing, path);
    return length < 0 || length >= PATH_MAX ? -ENAMETOOLONG : 0;
}
#define PATH(name, input) char name[PATH_MAX]; do { \
    int error = physical(input, name); if (error) return error; } while (0)
#define RESULT(call) do { if ((call) == -1) return -errno; return 0; } while (0)

static int attributes(const char *path, struct stat *value, struct fuse_file_info *fi) {
    (void)fi; PATH(full, path); RESULT(lstat(full, value));
}
static int directory(const char *path, mode_t mode) {
    PATH(full, path); RESULT(mkdir(full, mode));
}
static int remove_file(const char *path) {
    PATH(full, path); RESULT(unlink(full));
}
static int remove_directory(const char *path) {
    PATH(full, path); RESULT(rmdir(full));
}
static int move_file(const char *from, const char *to, unsigned flags) {
    if (flags) return -EINVAL;
    PATH(source, from); PATH(destination, to); RESULT(rename(source, destination));
}
static int open_file(const char *path, struct fuse_file_info *fi) {
    PATH(full, path); int fd = open(full, fi->flags);
    if (fd == -1) return -errno;
    fi->fh = (unsigned)fd; return 0;
}
static int create_file(const char *path, mode_t mode, struct fuse_file_info *fi) {
    PATH(full, path); int fd = open(full, fi->flags | O_CREAT, mode);
    if (fd == -1) return -errno;
    fi->fh = (unsigned)fd; return 0;
}
static int read_file(const char *path, char *buffer, size_t size, off_t offset, struct fuse_file_info *fi) {
    (void)path; ssize_t result = pread((int)fi->fh, buffer, size, offset);
    return result == -1 ? -errno : (int)result;
}
static int write_file(const char *path, const char *buffer, size_t size, off_t offset, struct fuse_file_info *fi) {
    (void)path; ssize_t result = pwrite((int)fi->fh, buffer, size, offset);
    return result == -1 ? -errno : (int)result;
}
static int close_file(const char *path, struct fuse_file_info *fi) {
    (void)path; RESULT(close((int)fi->fh));
}
static int resize_file(const char *path, off_t size, struct fuse_file_info *fi) {
    if (fi) { RESULT(ftruncate((int)fi->fh, size)); }
    PATH(full, path); RESULT(truncate(full, size));
}
static int sync_file(const char *path, int data_only, struct fuse_file_info *fi) {
    const char *filename = strrchr(path, '/');
    if (filename && strcmp(filename, "/transfer-events.tsv") == 0) {
        char enabled[PATH_MAX], observed[PATH_MAX];
        if (snprintf(enabled, sizeof(enabled), "%s/enabled", control) >= PATH_MAX ||
            snprintf(observed, sizeof(observed), "%s/observed", control) >= PATH_MAX) return -ENAMETOOLONG;
        if (access(enabled, F_OK) == 0) {
            int marker = open(observed, O_WRONLY | O_CREAT | O_TRUNC, 0600);
            if (marker == -1) return -errno;
            close(marker);
            struct timespec remaining = {.tv_sec = 0, .tv_nsec = 500000000};
            while (nanosleep(&remaining, &remaining) == -1 && errno == EINTR) {}
        }
    }
    RESULT(data_only ? fdatasync((int)fi->fh) : fsync((int)fi->fh));
}
static int sync_directory(const char *path, int data_only, struct fuse_file_info *fi) {
    (void)data_only; (void)fi; PATH(full, path);
    int fd = open(full, O_RDONLY | O_DIRECTORY);
    if (fd == -1) return -errno;
    int result = fsync(fd), saved = errno;
    close(fd); return result == -1 ? -saved : 0;
}
static int list_directory(const char *path, void *buffer, fuse_fill_dir_t fill,
                          off_t offset, struct fuse_file_info *fi, enum fuse_readdir_flags flags) {
    (void)offset; (void)fi; (void)flags; PATH(full, path);
    DIR *dir = opendir(full); if (!dir) return -errno;
    struct dirent *entry;
    while ((entry = readdir(dir))) {
        if (fill(buffer, entry->d_name, NULL, 0, 0)) break;
    }
    closedir(dir); return 0;
}
static int filesystem(const char *path, struct statvfs *value) {
    PATH(full, path); RESULT(statvfs(full, value));
}

int main(int argc, char **argv) {
    if (argc != 4) { fprintf(stderr, "usage: %s backing control mount\n", argv[0]); return 2; }
    backing = argv[1]; control = argv[2];
    struct fuse_operations operations = {
        .getattr = attributes, .mkdir = directory, .unlink = remove_file,
        .rmdir = remove_directory, .rename = move_file, .open = open_file,
        .create = create_file, .read = read_file, .write = write_file,
        .release = close_file, .truncate = resize_file, .fsync = sync_file,
        .fsyncdir = sync_directory, .readdir = list_directory, .statfs = filesystem,
    };
    char *arguments[] = {argv[0], "-f", "-s", argv[3], NULL};
    return fuse_main(4, arguments, &operations, NULL);
}
