package gg.strims.ppspp.rpc
import gg.strims.ppspp.proto.*

class ProfileClient(filepath: String) : RPCClient(filepath) {

    suspend fun create(
        arg: CreateProfileRequest = CreateProfileRequest()
    ): CreateProfileResponse =
        this.callUnary("Profile/Create", arg)

    suspend fun load(
        arg: LoadProfileRequest = LoadProfileRequest()
    ): LoadProfileResponse =
        this.callUnary("Profile/Load", arg)

    suspend fun get(
        arg: GetProfileRequest = GetProfileRequest()
    ): GetProfileResponse =
        this.callUnary("Profile/Get", arg)

    suspend fun update(
        arg: UpdateProfileRequest = UpdateProfileRequest()
    ): UpdateProfileResponse =
        this.callUnary("Profile/Update", arg)

    suspend fun delete(
        arg: DeleteProfileRequest = DeleteProfileRequest()
    ): DeleteProfileResponse =
        this.callUnary("Profile/Delete", arg)

    suspend fun list(
        arg: ListProfilesRequest = ListProfilesRequest()
    ): ListProfilesResponse =
        this.callUnary("Profile/List", arg)

    suspend fun loadSession(
        arg: LoadSessionRequest = LoadSessionRequest()
    ): LoadSessionResponse =
        this.callUnary("Profile/LoadSession", arg)

}
