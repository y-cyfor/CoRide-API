declare namespace Api {
  /**
   * namespace Auth
   *
   * backend api module: "auth"
   */
  namespace Auth {
    interface LoginToken {
      token: string;
      refreshToken?: string;
      user: {
        id: number;
        username: string;
        role: string;
      };
    }

    interface UserInfo {
      id: number;
      username: string;
      role: string;
    }
  }
}
