/* Minimal use of the uccharan C API.
 *
 *   cargo build --release -p uccharan-capi
 *   cc crates/uccharan-capi/examples/hello.c -Icrates/uccharan-capi/include \
 *      -Ltarget/release -luccharan -o hello
 *   LD_LIBRARY_PATH=target/release ./hello
 */
#include <stdio.h>
#include "uccharan.h"

int main(void) {
    UccharanPhonemizer *p = uccharan_new();
    char *ipa = uccharan_phonemize(p, "Hello, world.", "en-us", "espeak-ipa", 0);
    char *hi = uccharan_phonemize(p, "नमस्ते दुनिया", "hi", "piper", 0);
    if (!ipa || !hi) {
        fprintf(stderr, "phonemize failed\n");
        return 1;
    }
    printf("uccharan %s\n%s\n%s\n", uccharan_version(), ipa, hi);
    /* German has a word table that is not compiled in: it is read from
     * $UCCHARAN_DATA_DIR or the cache, and the call fails with a message that
     * names the pack when it is in neither. */
    char *de = uccharan_phonemize(p, "Guten Tag.", "de", "espeak-ipa", 0);
    if (de) {
        printf("%s\n", de);
    } else {
        printf("German: %s\n", uccharan_last_error());
    }
    uccharan_string_free(de);
    uccharan_string_free(ipa);
    uccharan_string_free(hi);
    uccharan_free(p);
    return 0;
}
