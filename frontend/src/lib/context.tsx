import {
  createContext,
  useContext,
  useEffect,
  useState,
  useRef,
  type ReactNode,
} from "react";
import { useAccount, useDisconnect } from "wagmi";
import { getAccount, watchAccount } from "wagmi/actions";
import {
  useConnectModal,
  useAccountModal,
  useChainModal,
} from "@rainbow-me/rainbowkit";
import { walletConfig } from "./rainbow";
import { useQueryClient } from "@tanstack/react-query";
import { demo, chain } from "./config";
import { post, request, setCsrf } from "./api";
import { demoWallet, loadDemo } from "./demo";
import { walletClient, switchChain, type Provider } from "./wallet";
import type { AdminProfile, SessionResult } from "./types";
type Session = { wallet: string; admin: boolean; provider: Provider | null };
type AppContextValue = {
  session: Session | null;
  connect: () => void;
  logout: () => Promise<void>;
  notify: (message: string) => void;
  manageWallet: () => void;
  changeNetwork: () => void;
};
const AppContext = createContext<AppContextValue>(null!);
export const useApp = () => useContext(AppContext);
export function AppProvider({ children }: { children: ReactNode }) {
  const query = useQueryClient();
  const account = useAccount();
  const { disconnectAsync } = useDisconnect();
  const { openConnectModal } = useConnectModal();
  const { openAccountModal } = useAccountModal();
  const { openChainModal } = useChainModal();
  const requested = useRef(false);
  const generation = useRef(0);
  const currentSession = useRef<Session | null>(null);
  const [session, setSession] = useState<Session | null>(null);
  const [open, setOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [toast, setToast] = useState("");
  currentSession.current = session;
  useEffect(() => {
    if (!demo && requested.current && account.isConnected) {
      requested.current = false;
      setOpen(true);
    }
  }, [account.isConnected]);
  useEffect(
    () =>
      watchAccount(walletConfig, {
        onChange(next, previous) {
          if (
            next.address !== previous.address ||
            next.chainId !== previous.chainId ||
            next.status !== previous.status
          ) {
            generation.current++;
            const active = currentSession.current;
            if (
              active &&
              !demo &&
              (next.address?.toLowerCase() !== active.wallet.toLowerCase() ||
                next.chainId !== chain.id ||
                next.status !== "connected")
            ) {
              setSession(null);
              const revoke = post("/auth/logout");
              setCsrf(null);
              query.clear();
              void revoke.catch(() => {});
              setToast("Wallet changed. Sign in again to continue.");
            }
          }
        },
      }),
    [query],
  );
  useEffect(() => {
    if (demo) loadDemo();
  }, []);
  useEffect(() => {
    if (!toast) return;
    const timer = setTimeout(() => setToast(""), 6000);
    return () => clearTimeout(timer);
  }, [toast]);
  async function logout() {
    const previous = session;
    generation.current++;
    requested.current = false;
    setOpen(false);
    setSession(null);
    query.clear();
    const revocation =
      previous && !demo ? post("/auth/logout") : Promise.resolve();
    setCsrf(null);
    if (!demo) await disconnectAsync().catch(() => {});
    try {
      await revocation;
    } catch {
      /* Local logout still clears privileged UI. */
    }
  }
  async function connectWallet() {
    setBusy(true);
    setError("");
    try {
      const connected = getAccount(walletConfig);
      const provider = (await connected.connector?.getProvider()) as
        Provider | undefined;
      if (!provider)
        throw new Error("Choose and connect a wallet before signing in.");
      const client = walletClient(provider);
      const [wallet] = await client.getAddresses();
      if (!wallet) throw new Error("No wallet account selected.");
      if ((await client.getChainId()) !== chain.id) await switchChain(provider);
      const attempt = generation.current;
      const challenge = await post<{ message: string; expires_at: string }>(
        "/auth/challenge",
        { wallet, chain_id: String(chain.id) },
      );
      const signature = await client.signMessage({
        account: wallet,
        message: challenge.message,
      });
      const [addresses, network] = await Promise.all([
        client.getAddresses(),
        client.getChainId(),
      ]);
      if (
        attempt !== generation.current ||
        addresses[0]?.toLowerCase() !== wallet.toLowerCase() ||
        network !== chain.id
      )
        throw new Error("Wallet changed while signing. Start again.");
      const verified = await post<SessionResult>("/auth/verify", {
        message: challenge.message,
        signature,
      });
      if (
        attempt !== generation.current ||
        verified.wallet.toLowerCase() !== wallet.toLowerCase()
      ) {
        setCsrf(verified.csrf_token);
        const revoke = post("/auth/logout");
        setCsrf(null);
        void revoke.catch(() => {});
        throw new Error("Wallet changed during verification. Start again.");
      }
      setCsrf(verified.csrf_token);
      let admin = false;
      try {
        await request<AdminProfile>("/admin/me");
        admin = true;
      } catch {
        /* Admin is granted only by the server. */
      }
      if (attempt !== generation.current) {
        const revoke = post("/auth/logout");
        setCsrf(null);
        void revoke.catch(() => {});
        throw new Error("Wallet changed during sign-in. Start again.");
      }
      setSession({ wallet: verified.wallet, admin, provider });
      setOpen(false);
      query.clear();
      setToast("Wallet connected. You are signed in.");
    } catch (e) {
      setError(e instanceof Error ? e.message : "Connection failed.");
    } finally {
      setBusy(false);
    }
  }
  return (
    <AppContext.Provider
      value={{
        session,
        connect: () => {
          setError("");
          if (demo || account.isConnected) setOpen(true);
          else {
            requested.current = true;
            openConnectModal?.();
          }
        },
        manageWallet: () => openAccountModal?.(),
        changeNetwork: () => openChainModal?.(),
        logout,
        notify: setToast,
      }}
    >
      {children}
      {toast && (
        <div className="toast" role="status">
          {toast}
          <button
            aria-label="Dismiss notification"
            onClick={() => setToast("")}
          >
            ×
          </button>
        </div>
      )}
      {open && (
        <ConnectDialog close={() => setOpen(false)}>
          <h2 id="dialog-title">
            {demo ? "Explore Oppor" : "Connect your wallet"}
          </h2>
          <p>
            {demo
              ? "You are viewing sample campaigns. Try the participant and creator flows without using real funds."
              : "Sign in with your wallet. Signing a message does not authorize a token transfer."}
          </p>
          {error && (
            <p className="error" role="alert">
              {error}
            </p>
          )}
          {demo ? (
            <button
              className="button primary wide"
              onClick={() => {
                setSession({
                  wallet: demoWallet,
                  admin: false,
                  provider: null,
                });
                setOpen(false);
                query.clear();
              }}
            >
              Continue in demo mode
            </button>
          ) : (
            <button
              className="button primary wide"
              disabled={busy}
              onClick={() => void connectWallet()}
            >
              {busy ? "Waiting for wallet…" : "Sign in to Oppor"}
            </button>
          )}
          {!demo && (
            <button
              className="button outline wide"
              disabled={busy}
              onClick={() => {
                setOpen(false);
                void disconnectAsync()
                  .then(() => {
                    requested.current = true;
                    openConnectModal?.();
                  })
                  .catch(() => setError("Unable to disconnect wallet."));
              }}
            >
              Choose another wallet
            </button>
          )}
          {demo && (
            <button
              className="button outline wide"
              onClick={() => {
                setOpen(false);
                openConnectModal?.();
              }}
            >
              Connect a wallet
            </button>
          )}
          <p className="fineprint">
            {demo
              ? "No blockchain transactions will be sent."
              : "You will be asked to switch to the configured Arc network. Reconnect after a page reload to authorize changes."}
          </p>
        </ConnectDialog>
      )}
    </AppContext.Provider>
  );
}
function ConnectDialog({
  close,
  children,
}: {
  close: () => void;
  children: ReactNode;
}) {
  return <Dialog close={close}>{children}</Dialog>;
}
export function Dialog({
  close,
  children,
}: {
  close: () => void;
  children: ReactNode;
}) {
  const closeRef = useRef(close);
  closeRef.current = close;
  useEffect(() => {
    const previous = document.activeElement as HTMLElement | null;
    const box = document.getElementById("dialog");
    const focusable = () =>
      Array.from(
        box?.querySelectorAll<HTMLElement>(
          'button:not(:disabled),a[href],input,select,textarea,[tabindex="0"]',
        ) || [],
      );
    focusable()[0]?.focus();
    const key = (event: KeyboardEvent) => {
      if (event.key === "Escape") closeRef.current();
      if (event.key === "Tab") {
        const items = focusable(),
          first = items[0],
          last = items.at(-1);
        if (event.shiftKey && document.activeElement === first) {
          event.preventDefault();
          last?.focus();
        } else if (!event.shiftKey && document.activeElement === last) {
          event.preventDefault();
          first?.focus();
        }
      }
    };
    document.addEventListener("keydown", key);
    const old = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    return () => {
      document.removeEventListener("keydown", key);
      document.body.style.overflow = old;
      previous?.focus();
    };
  }, []);
  return (
    <div
      className="modal-backdrop"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) close();
      }}
    >
      <section
        id="dialog"
        className="dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="dialog-title"
      >
        <button
          className="icon-button close"
          aria-label="Close dialog"
          onClick={close}
        >
          ×
        </button>
        {children}
      </section>
    </div>
  );
}
