#include "vdjOnlineSource.h"
#include <cstddef>
#include <cstdint>
#include <cstdio>
#include <cstring>

struct PMF { uintptr_t ptr; ptrdiff_t adj; };
template <typename M> long slot(M m) {
    PMF p{}; static_assert(sizeof(M) == sizeof(p)); std::memcpy(&p, &m, sizeof(p));
    if (p.adj & 1) return p.ptr / sizeof(void*);
    if (p.ptr & 1) return (p.ptr - 1) / sizeof(void*);
    return -1;
}
#define SLOT(name, cls, method) std::printf(#name " %ld\n", slot(&cls::method))
int main() {
    std::printf("size %zu\n", sizeof(IVdjPluginOnlineSource));
    std::printf("oauth_offset %zu\n", offsetof(IVdjPluginOnlineSource, oauth));
    SLOT(is_logged, IVdjPluginOnlineSource, IsLogged);
    SLOT(on_oauth, IVdjPluginOnlineSource, OnOAuth);
    SLOT(on_search, IVdjPluginOnlineSource, OnSearch);
    SLOT(on_search_cancel, IVdjPluginOnlineSource, OnSearchCancel);
    SLOT(get_stream_url, IVdjPluginOnlineSource, GetStreamUrl);
    SLOT(get_folder_list, IVdjPluginOnlineSource, GetFolderList);
    SLOT(get_folder, IVdjPluginOnlineSource, GetFolder);
    SLOT(get_context_menu, IVdjPluginOnlineSource, GetContextMenu);
    SLOT(on_context_menu, IVdjPluginOnlineSource, OnContextMenu);
    SLOT(get_folder_context_menu, IVdjPluginOnlineSource, GetFolderContextMenu);
    SLOT(on_folder_context_menu, IVdjPluginOnlineSource, OnFolderContextMenu);
    SLOT(tracks_add, IVdjTracksList, add);
    SLOT(tracks_finish, IVdjTracksList, finish);
    std::printf("string_assign %ld\n", slot(static_cast<void (IVdjString::*)(const char*)>(&IVdjString::operator=)));
}
