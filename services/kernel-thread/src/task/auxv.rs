#[derive(PartialEq, Eq, PartialOrd, Ord)]
#[allow(non_camel_case_types, dead_code)]
#[allow(clippy::upper_case_acronyms)]
pub enum AuxV {
    /// end of vector
    NULL = 0,
    /// entry should be ignored
    IGNORE = 1,
    /// file descriptor of program
    EXECFD = 2,
    /// program headers for program
    PHDR = 3,
    /// size of program header entry
    PHENT = 4,
    /// number of program headers
    PHNUM = 5,
    /// system page size
    PAGESZ = 6,
    /// base address of interpreter
    BASE = 7,
    /// flags
    FLAGS = 8,
    /// entry point of program
    ENTRY = 9,
    /// program is not ELF
    NOTELF = 10,
    /// real uid
    UID = 11,
    /// effective uid
    EUID = 12,
    /// real gid
    GID = 13,
    /// effective gid
    EGID = 14,
    /// string identifying CPU for optimizations
    PLATFORM = 15,
    /// arch dependent hints at CPU capabilities
    HWCAP = 16,
    /// frequency at which times() increments
    CLKTCK = 17,
    // values 18 through 22 are reserved
    DCACHEBSIZE = 19,
    /// secure mode boolean
    SECURE = 23,
    /// string identifying real platform, may differ from AT_PLATFORM
    BASE_PLATFORM = 24,
    /// address of 16 random bytes
    RANDOM = 25,
    /// extension of AT_HWCAP
    HWCAP2 = 26,
    /// filename of program
    EXECFN = 31,
}
