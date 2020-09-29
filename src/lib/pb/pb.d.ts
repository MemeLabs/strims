import * as $protobuf from "protobufjs";
/** Properties of a JoinSwarmRequest. */
export interface IJoinSwarmRequest {

    /** JoinSwarmRequest swarmUri */
    swarmUri?: (string|null);
}

/** Represents a JoinSwarmRequest. */
export class JoinSwarmRequest implements IJoinSwarmRequest {

    /**
     * Constructs a new JoinSwarmRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: IJoinSwarmRequest);

    /** JoinSwarmRequest swarmUri. */
    public swarmUri: string;

    /**
     * Creates a new JoinSwarmRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns JoinSwarmRequest instance
     */
    public static create(properties?: IJoinSwarmRequest): JoinSwarmRequest;

    /**
     * Encodes the specified JoinSwarmRequest message. Does not implicitly {@link JoinSwarmRequest.verify|verify} messages.
     * @param message JoinSwarmRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IJoinSwarmRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified JoinSwarmRequest message, length delimited. Does not implicitly {@link JoinSwarmRequest.verify|verify} messages.
     * @param message JoinSwarmRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IJoinSwarmRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a JoinSwarmRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns JoinSwarmRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): JoinSwarmRequest;

    /**
     * Decodes a JoinSwarmRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns JoinSwarmRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): JoinSwarmRequest;

    /**
     * Verifies a JoinSwarmRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a JoinSwarmRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns JoinSwarmRequest
     */
    public static fromObject(object: { [k: string]: any }): JoinSwarmRequest;

    /**
     * Creates a plain object from a JoinSwarmRequest message. Also converts values to other types if specified.
     * @param message JoinSwarmRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: JoinSwarmRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this JoinSwarmRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a JoinSwarmResponse. */
export interface IJoinSwarmResponse {
}

/** Represents a JoinSwarmResponse. */
export class JoinSwarmResponse implements IJoinSwarmResponse {

    /**
     * Constructs a new JoinSwarmResponse.
     * @param [properties] Properties to set
     */
    constructor(properties?: IJoinSwarmResponse);

    /**
     * Creates a new JoinSwarmResponse instance using the specified properties.
     * @param [properties] Properties to set
     * @returns JoinSwarmResponse instance
     */
    public static create(properties?: IJoinSwarmResponse): JoinSwarmResponse;

    /**
     * Encodes the specified JoinSwarmResponse message. Does not implicitly {@link JoinSwarmResponse.verify|verify} messages.
     * @param message JoinSwarmResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IJoinSwarmResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified JoinSwarmResponse message, length delimited. Does not implicitly {@link JoinSwarmResponse.verify|verify} messages.
     * @param message JoinSwarmResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IJoinSwarmResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a JoinSwarmResponse message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns JoinSwarmResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): JoinSwarmResponse;

    /**
     * Decodes a JoinSwarmResponse message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns JoinSwarmResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): JoinSwarmResponse;

    /**
     * Verifies a JoinSwarmResponse message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a JoinSwarmResponse message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns JoinSwarmResponse
     */
    public static fromObject(object: { [k: string]: any }): JoinSwarmResponse;

    /**
     * Creates a plain object from a JoinSwarmResponse message. Also converts values to other types if specified.
     * @param message JoinSwarmResponse
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: JoinSwarmResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this JoinSwarmResponse to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a LeaveSwarmRequest. */
export interface ILeaveSwarmRequest {

    /** LeaveSwarmRequest swarmUri */
    swarmUri?: (string|null);
}

/** Represents a LeaveSwarmRequest. */
export class LeaveSwarmRequest implements ILeaveSwarmRequest {

    /**
     * Constructs a new LeaveSwarmRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: ILeaveSwarmRequest);

    /** LeaveSwarmRequest swarmUri. */
    public swarmUri: string;

    /**
     * Creates a new LeaveSwarmRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns LeaveSwarmRequest instance
     */
    public static create(properties?: ILeaveSwarmRequest): LeaveSwarmRequest;

    /**
     * Encodes the specified LeaveSwarmRequest message. Does not implicitly {@link LeaveSwarmRequest.verify|verify} messages.
     * @param message LeaveSwarmRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: ILeaveSwarmRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified LeaveSwarmRequest message, length delimited. Does not implicitly {@link LeaveSwarmRequest.verify|verify} messages.
     * @param message LeaveSwarmRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: ILeaveSwarmRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a LeaveSwarmRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns LeaveSwarmRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): LeaveSwarmRequest;

    /**
     * Decodes a LeaveSwarmRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns LeaveSwarmRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): LeaveSwarmRequest;

    /**
     * Verifies a LeaveSwarmRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a LeaveSwarmRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns LeaveSwarmRequest
     */
    public static fromObject(object: { [k: string]: any }): LeaveSwarmRequest;

    /**
     * Creates a plain object from a LeaveSwarmRequest message. Also converts values to other types if specified.
     * @param message LeaveSwarmRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: LeaveSwarmRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this LeaveSwarmRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a LeaveSwarmResponse. */
export interface ILeaveSwarmResponse {
}

/** Represents a LeaveSwarmResponse. */
export class LeaveSwarmResponse implements ILeaveSwarmResponse {

    /**
     * Constructs a new LeaveSwarmResponse.
     * @param [properties] Properties to set
     */
    constructor(properties?: ILeaveSwarmResponse);

    /**
     * Creates a new LeaveSwarmResponse instance using the specified properties.
     * @param [properties] Properties to set
     * @returns LeaveSwarmResponse instance
     */
    public static create(properties?: ILeaveSwarmResponse): LeaveSwarmResponse;

    /**
     * Encodes the specified LeaveSwarmResponse message. Does not implicitly {@link LeaveSwarmResponse.verify|verify} messages.
     * @param message LeaveSwarmResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: ILeaveSwarmResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified LeaveSwarmResponse message, length delimited. Does not implicitly {@link LeaveSwarmResponse.verify|verify} messages.
     * @param message LeaveSwarmResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: ILeaveSwarmResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a LeaveSwarmResponse message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns LeaveSwarmResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): LeaveSwarmResponse;

    /**
     * Decodes a LeaveSwarmResponse message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns LeaveSwarmResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): LeaveSwarmResponse;

    /**
     * Verifies a LeaveSwarmResponse message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a LeaveSwarmResponse message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns LeaveSwarmResponse
     */
    public static fromObject(object: { [k: string]: any }): LeaveSwarmResponse;

    /**
     * Creates a plain object from a LeaveSwarmResponse message. Also converts values to other types if specified.
     * @param message LeaveSwarmResponse
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: LeaveSwarmResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this LeaveSwarmResponse to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a GetIngressStreamsRequest. */
export interface IGetIngressStreamsRequest {
}

/** Represents a GetIngressStreamsRequest. */
export class GetIngressStreamsRequest implements IGetIngressStreamsRequest {

    /**
     * Constructs a new GetIngressStreamsRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: IGetIngressStreamsRequest);

    /**
     * Creates a new GetIngressStreamsRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns GetIngressStreamsRequest instance
     */
    public static create(properties?: IGetIngressStreamsRequest): GetIngressStreamsRequest;

    /**
     * Encodes the specified GetIngressStreamsRequest message. Does not implicitly {@link GetIngressStreamsRequest.verify|verify} messages.
     * @param message GetIngressStreamsRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IGetIngressStreamsRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified GetIngressStreamsRequest message, length delimited. Does not implicitly {@link GetIngressStreamsRequest.verify|verify} messages.
     * @param message GetIngressStreamsRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IGetIngressStreamsRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a GetIngressStreamsRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns GetIngressStreamsRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): GetIngressStreamsRequest;

    /**
     * Decodes a GetIngressStreamsRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns GetIngressStreamsRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): GetIngressStreamsRequest;

    /**
     * Verifies a GetIngressStreamsRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a GetIngressStreamsRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns GetIngressStreamsRequest
     */
    public static fromObject(object: { [k: string]: any }): GetIngressStreamsRequest;

    /**
     * Creates a plain object from a GetIngressStreamsRequest message. Also converts values to other types if specified.
     * @param message GetIngressStreamsRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: GetIngressStreamsRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this GetIngressStreamsRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a GetIngressStreamsResponse. */
export interface IGetIngressStreamsResponse {

    /** GetIngressStreamsResponse swarmUri */
    swarmUri?: (string|null);
}

/** Represents a GetIngressStreamsResponse. */
export class GetIngressStreamsResponse implements IGetIngressStreamsResponse {

    /**
     * Constructs a new GetIngressStreamsResponse.
     * @param [properties] Properties to set
     */
    constructor(properties?: IGetIngressStreamsResponse);

    /** GetIngressStreamsResponse swarmUri. */
    public swarmUri: string;

    /**
     * Creates a new GetIngressStreamsResponse instance using the specified properties.
     * @param [properties] Properties to set
     * @returns GetIngressStreamsResponse instance
     */
    public static create(properties?: IGetIngressStreamsResponse): GetIngressStreamsResponse;

    /**
     * Encodes the specified GetIngressStreamsResponse message. Does not implicitly {@link GetIngressStreamsResponse.verify|verify} messages.
     * @param message GetIngressStreamsResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IGetIngressStreamsResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified GetIngressStreamsResponse message, length delimited. Does not implicitly {@link GetIngressStreamsResponse.verify|verify} messages.
     * @param message GetIngressStreamsResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IGetIngressStreamsResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a GetIngressStreamsResponse message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns GetIngressStreamsResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): GetIngressStreamsResponse;

    /**
     * Decodes a GetIngressStreamsResponse message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns GetIngressStreamsResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): GetIngressStreamsResponse;

    /**
     * Verifies a GetIngressStreamsResponse message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a GetIngressStreamsResponse message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns GetIngressStreamsResponse
     */
    public static fromObject(object: { [k: string]: any }): GetIngressStreamsResponse;

    /**
     * Creates a plain object from a GetIngressStreamsResponse message. Also converts values to other types if specified.
     * @param message GetIngressStreamsResponse
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: GetIngressStreamsResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this GetIngressStreamsResponse to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a StartRTMPIngressRequest. */
export interface IStartRTMPIngressRequest {
}

/** Represents a StartRTMPIngressRequest. */
export class StartRTMPIngressRequest implements IStartRTMPIngressRequest {

    /**
     * Constructs a new StartRTMPIngressRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: IStartRTMPIngressRequest);

    /**
     * Creates a new StartRTMPIngressRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns StartRTMPIngressRequest instance
     */
    public static create(properties?: IStartRTMPIngressRequest): StartRTMPIngressRequest;

    /**
     * Encodes the specified StartRTMPIngressRequest message. Does not implicitly {@link StartRTMPIngressRequest.verify|verify} messages.
     * @param message StartRTMPIngressRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IStartRTMPIngressRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified StartRTMPIngressRequest message, length delimited. Does not implicitly {@link StartRTMPIngressRequest.verify|verify} messages.
     * @param message StartRTMPIngressRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IStartRTMPIngressRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a StartRTMPIngressRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns StartRTMPIngressRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): StartRTMPIngressRequest;

    /**
     * Decodes a StartRTMPIngressRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns StartRTMPIngressRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): StartRTMPIngressRequest;

    /**
     * Verifies a StartRTMPIngressRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a StartRTMPIngressRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns StartRTMPIngressRequest
     */
    public static fromObject(object: { [k: string]: any }): StartRTMPIngressRequest;

    /**
     * Creates a plain object from a StartRTMPIngressRequest message. Also converts values to other types if specified.
     * @param message StartRTMPIngressRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: StartRTMPIngressRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this StartRTMPIngressRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a StartRTMPIngressResponse. */
export interface IStartRTMPIngressResponse {
}

/** Represents a StartRTMPIngressResponse. */
export class StartRTMPIngressResponse implements IStartRTMPIngressResponse {

    /**
     * Constructs a new StartRTMPIngressResponse.
     * @param [properties] Properties to set
     */
    constructor(properties?: IStartRTMPIngressResponse);

    /**
     * Creates a new StartRTMPIngressResponse instance using the specified properties.
     * @param [properties] Properties to set
     * @returns StartRTMPIngressResponse instance
     */
    public static create(properties?: IStartRTMPIngressResponse): StartRTMPIngressResponse;

    /**
     * Encodes the specified StartRTMPIngressResponse message. Does not implicitly {@link StartRTMPIngressResponse.verify|verify} messages.
     * @param message StartRTMPIngressResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IStartRTMPIngressResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified StartRTMPIngressResponse message, length delimited. Does not implicitly {@link StartRTMPIngressResponse.verify|verify} messages.
     * @param message StartRTMPIngressResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IStartRTMPIngressResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a StartRTMPIngressResponse message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns StartRTMPIngressResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): StartRTMPIngressResponse;

    /**
     * Decodes a StartRTMPIngressResponse message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns StartRTMPIngressResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): StartRTMPIngressResponse;

    /**
     * Verifies a StartRTMPIngressResponse message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a StartRTMPIngressResponse message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns StartRTMPIngressResponse
     */
    public static fromObject(object: { [k: string]: any }): StartRTMPIngressResponse;

    /**
     * Creates a plain object from a StartRTMPIngressResponse message. Also converts values to other types if specified.
     * @param message StartRTMPIngressResponse
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: StartRTMPIngressResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this StartRTMPIngressResponse to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a StartHLSEgressRequest. */
export interface IStartHLSEgressRequest {

    /** StartHLSEgressRequest videoId */
    videoId?: (number|null);

    /** StartHLSEgressRequest address */
    address?: (string|null);
}

/** Represents a StartHLSEgressRequest. */
export class StartHLSEgressRequest implements IStartHLSEgressRequest {

    /**
     * Constructs a new StartHLSEgressRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: IStartHLSEgressRequest);

    /** StartHLSEgressRequest videoId. */
    public videoId: number;

    /** StartHLSEgressRequest address. */
    public address: string;

    /**
     * Creates a new StartHLSEgressRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns StartHLSEgressRequest instance
     */
    public static create(properties?: IStartHLSEgressRequest): StartHLSEgressRequest;

    /**
     * Encodes the specified StartHLSEgressRequest message. Does not implicitly {@link StartHLSEgressRequest.verify|verify} messages.
     * @param message StartHLSEgressRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IStartHLSEgressRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified StartHLSEgressRequest message, length delimited. Does not implicitly {@link StartHLSEgressRequest.verify|verify} messages.
     * @param message StartHLSEgressRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IStartHLSEgressRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a StartHLSEgressRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns StartHLSEgressRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): StartHLSEgressRequest;

    /**
     * Decodes a StartHLSEgressRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns StartHLSEgressRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): StartHLSEgressRequest;

    /**
     * Verifies a StartHLSEgressRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a StartHLSEgressRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns StartHLSEgressRequest
     */
    public static fromObject(object: { [k: string]: any }): StartHLSEgressRequest;

    /**
     * Creates a plain object from a StartHLSEgressRequest message. Also converts values to other types if specified.
     * @param message StartHLSEgressRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: StartHLSEgressRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this StartHLSEgressRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a StartHLSEgressResponse. */
export interface IStartHLSEgressResponse {

    /** StartHLSEgressResponse id */
    id?: (number|null);

    /** StartHLSEgressResponse url */
    url?: (string|null);
}

/** Represents a StartHLSEgressResponse. */
export class StartHLSEgressResponse implements IStartHLSEgressResponse {

    /**
     * Constructs a new StartHLSEgressResponse.
     * @param [properties] Properties to set
     */
    constructor(properties?: IStartHLSEgressResponse);

    /** StartHLSEgressResponse id. */
    public id: number;

    /** StartHLSEgressResponse url. */
    public url: string;

    /**
     * Creates a new StartHLSEgressResponse instance using the specified properties.
     * @param [properties] Properties to set
     * @returns StartHLSEgressResponse instance
     */
    public static create(properties?: IStartHLSEgressResponse): StartHLSEgressResponse;

    /**
     * Encodes the specified StartHLSEgressResponse message. Does not implicitly {@link StartHLSEgressResponse.verify|verify} messages.
     * @param message StartHLSEgressResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IStartHLSEgressResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified StartHLSEgressResponse message, length delimited. Does not implicitly {@link StartHLSEgressResponse.verify|verify} messages.
     * @param message StartHLSEgressResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IStartHLSEgressResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a StartHLSEgressResponse message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns StartHLSEgressResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): StartHLSEgressResponse;

    /**
     * Decodes a StartHLSEgressResponse message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns StartHLSEgressResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): StartHLSEgressResponse;

    /**
     * Verifies a StartHLSEgressResponse message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a StartHLSEgressResponse message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns StartHLSEgressResponse
     */
    public static fromObject(object: { [k: string]: any }): StartHLSEgressResponse;

    /**
     * Creates a plain object from a StartHLSEgressResponse message. Also converts values to other types if specified.
     * @param message StartHLSEgressResponse
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: StartHLSEgressResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this StartHLSEgressResponse to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a StopHLSEgressRequest. */
export interface IStopHLSEgressRequest {

    /** StopHLSEgressRequest id */
    id?: (number|null);
}

/** Represents a StopHLSEgressRequest. */
export class StopHLSEgressRequest implements IStopHLSEgressRequest {

    /**
     * Constructs a new StopHLSEgressRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: IStopHLSEgressRequest);

    /** StopHLSEgressRequest id. */
    public id: number;

    /**
     * Creates a new StopHLSEgressRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns StopHLSEgressRequest instance
     */
    public static create(properties?: IStopHLSEgressRequest): StopHLSEgressRequest;

    /**
     * Encodes the specified StopHLSEgressRequest message. Does not implicitly {@link StopHLSEgressRequest.verify|verify} messages.
     * @param message StopHLSEgressRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IStopHLSEgressRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified StopHLSEgressRequest message, length delimited. Does not implicitly {@link StopHLSEgressRequest.verify|verify} messages.
     * @param message StopHLSEgressRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IStopHLSEgressRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a StopHLSEgressRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns StopHLSEgressRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): StopHLSEgressRequest;

    /**
     * Decodes a StopHLSEgressRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns StopHLSEgressRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): StopHLSEgressRequest;

    /**
     * Verifies a StopHLSEgressRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a StopHLSEgressRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns StopHLSEgressRequest
     */
    public static fromObject(object: { [k: string]: any }): StopHLSEgressRequest;

    /**
     * Creates a plain object from a StopHLSEgressRequest message. Also converts values to other types if specified.
     * @param message StopHLSEgressRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: StopHLSEgressRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this StopHLSEgressRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a StopHLSEgressResponse. */
export interface IStopHLSEgressResponse {
}

/** Represents a StopHLSEgressResponse. */
export class StopHLSEgressResponse implements IStopHLSEgressResponse {

    /**
     * Constructs a new StopHLSEgressResponse.
     * @param [properties] Properties to set
     */
    constructor(properties?: IStopHLSEgressResponse);

    /**
     * Creates a new StopHLSEgressResponse instance using the specified properties.
     * @param [properties] Properties to set
     * @returns StopHLSEgressResponse instance
     */
    public static create(properties?: IStopHLSEgressResponse): StopHLSEgressResponse;

    /**
     * Encodes the specified StopHLSEgressResponse message. Does not implicitly {@link StopHLSEgressResponse.verify|verify} messages.
     * @param message StopHLSEgressResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IStopHLSEgressResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified StopHLSEgressResponse message, length delimited. Does not implicitly {@link StopHLSEgressResponse.verify|verify} messages.
     * @param message StopHLSEgressResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IStopHLSEgressResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a StopHLSEgressResponse message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns StopHLSEgressResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): StopHLSEgressResponse;

    /**
     * Decodes a StopHLSEgressResponse message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns StopHLSEgressResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): StopHLSEgressResponse;

    /**
     * Verifies a StopHLSEgressResponse message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a StopHLSEgressResponse message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns StopHLSEgressResponse
     */
    public static fromObject(object: { [k: string]: any }): StopHLSEgressResponse;

    /**
     * Creates a plain object from a StopHLSEgressResponse message. Also converts values to other types if specified.
     * @param message StopHLSEgressResponse
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: StopHLSEgressResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this StopHLSEgressResponse to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a StartSwarmRequest. */
export interface IStartSwarmRequest {
}

/** Represents a StartSwarmRequest. */
export class StartSwarmRequest implements IStartSwarmRequest {

    /**
     * Constructs a new StartSwarmRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: IStartSwarmRequest);

    /**
     * Creates a new StartSwarmRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns StartSwarmRequest instance
     */
    public static create(properties?: IStartSwarmRequest): StartSwarmRequest;

    /**
     * Encodes the specified StartSwarmRequest message. Does not implicitly {@link StartSwarmRequest.verify|verify} messages.
     * @param message StartSwarmRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IStartSwarmRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified StartSwarmRequest message, length delimited. Does not implicitly {@link StartSwarmRequest.verify|verify} messages.
     * @param message StartSwarmRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IStartSwarmRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a StartSwarmRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns StartSwarmRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): StartSwarmRequest;

    /**
     * Decodes a StartSwarmRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns StartSwarmRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): StartSwarmRequest;

    /**
     * Verifies a StartSwarmRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a StartSwarmRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns StartSwarmRequest
     */
    public static fromObject(object: { [k: string]: any }): StartSwarmRequest;

    /**
     * Creates a plain object from a StartSwarmRequest message. Also converts values to other types if specified.
     * @param message StartSwarmRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: StartSwarmRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this StartSwarmRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a StartSwarmResponse. */
export interface IStartSwarmResponse {

    /** StartSwarmResponse id */
    id?: (number|null);
}

/** Represents a StartSwarmResponse. */
export class StartSwarmResponse implements IStartSwarmResponse {

    /**
     * Constructs a new StartSwarmResponse.
     * @param [properties] Properties to set
     */
    constructor(properties?: IStartSwarmResponse);

    /** StartSwarmResponse id. */
    public id: number;

    /**
     * Creates a new StartSwarmResponse instance using the specified properties.
     * @param [properties] Properties to set
     * @returns StartSwarmResponse instance
     */
    public static create(properties?: IStartSwarmResponse): StartSwarmResponse;

    /**
     * Encodes the specified StartSwarmResponse message. Does not implicitly {@link StartSwarmResponse.verify|verify} messages.
     * @param message StartSwarmResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IStartSwarmResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified StartSwarmResponse message, length delimited. Does not implicitly {@link StartSwarmResponse.verify|verify} messages.
     * @param message StartSwarmResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IStartSwarmResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a StartSwarmResponse message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns StartSwarmResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): StartSwarmResponse;

    /**
     * Decodes a StartSwarmResponse message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns StartSwarmResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): StartSwarmResponse;

    /**
     * Verifies a StartSwarmResponse message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a StartSwarmResponse message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns StartSwarmResponse
     */
    public static fromObject(object: { [k: string]: any }): StartSwarmResponse;

    /**
     * Creates a plain object from a StartSwarmResponse message. Also converts values to other types if specified.
     * @param message StartSwarmResponse
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: StartSwarmResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this StartSwarmResponse to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a WriteToSwarmRequest. */
export interface IWriteToSwarmRequest {

    /** WriteToSwarmRequest id */
    id?: (number|null);

    /** WriteToSwarmRequest data */
    data?: (Uint8Array|null);
}

/** Represents a WriteToSwarmRequest. */
export class WriteToSwarmRequest implements IWriteToSwarmRequest {

    /**
     * Constructs a new WriteToSwarmRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: IWriteToSwarmRequest);

    /** WriteToSwarmRequest id. */
    public id: number;

    /** WriteToSwarmRequest data. */
    public data: Uint8Array;

    /**
     * Creates a new WriteToSwarmRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns WriteToSwarmRequest instance
     */
    public static create(properties?: IWriteToSwarmRequest): WriteToSwarmRequest;

    /**
     * Encodes the specified WriteToSwarmRequest message. Does not implicitly {@link WriteToSwarmRequest.verify|verify} messages.
     * @param message WriteToSwarmRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IWriteToSwarmRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified WriteToSwarmRequest message, length delimited. Does not implicitly {@link WriteToSwarmRequest.verify|verify} messages.
     * @param message WriteToSwarmRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IWriteToSwarmRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a WriteToSwarmRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns WriteToSwarmRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): WriteToSwarmRequest;

    /**
     * Decodes a WriteToSwarmRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns WriteToSwarmRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): WriteToSwarmRequest;

    /**
     * Verifies a WriteToSwarmRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a WriteToSwarmRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns WriteToSwarmRequest
     */
    public static fromObject(object: { [k: string]: any }): WriteToSwarmRequest;

    /**
     * Creates a plain object from a WriteToSwarmRequest message. Also converts values to other types if specified.
     * @param message WriteToSwarmRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: WriteToSwarmRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this WriteToSwarmRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a WriteToSwarmResponse. */
export interface IWriteToSwarmResponse {

    /** WriteToSwarmResponse error */
    error?: (string|null);
}

/** Represents a WriteToSwarmResponse. */
export class WriteToSwarmResponse implements IWriteToSwarmResponse {

    /**
     * Constructs a new WriteToSwarmResponse.
     * @param [properties] Properties to set
     */
    constructor(properties?: IWriteToSwarmResponse);

    /** WriteToSwarmResponse error. */
    public error: string;

    /**
     * Creates a new WriteToSwarmResponse instance using the specified properties.
     * @param [properties] Properties to set
     * @returns WriteToSwarmResponse instance
     */
    public static create(properties?: IWriteToSwarmResponse): WriteToSwarmResponse;

    /**
     * Encodes the specified WriteToSwarmResponse message. Does not implicitly {@link WriteToSwarmResponse.verify|verify} messages.
     * @param message WriteToSwarmResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IWriteToSwarmResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified WriteToSwarmResponse message, length delimited. Does not implicitly {@link WriteToSwarmResponse.verify|verify} messages.
     * @param message WriteToSwarmResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IWriteToSwarmResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a WriteToSwarmResponse message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns WriteToSwarmResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): WriteToSwarmResponse;

    /**
     * Decodes a WriteToSwarmResponse message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns WriteToSwarmResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): WriteToSwarmResponse;

    /**
     * Verifies a WriteToSwarmResponse message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a WriteToSwarmResponse message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns WriteToSwarmResponse
     */
    public static fromObject(object: { [k: string]: any }): WriteToSwarmResponse;

    /**
     * Creates a plain object from a WriteToSwarmResponse message. Also converts values to other types if specified.
     * @param message WriteToSwarmResponse
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: WriteToSwarmResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this WriteToSwarmResponse to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a StopSwarmRequest. */
export interface IStopSwarmRequest {

    /** StopSwarmRequest id */
    id?: (number|null);
}

/** Represents a StopSwarmRequest. */
export class StopSwarmRequest implements IStopSwarmRequest {

    /**
     * Constructs a new StopSwarmRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: IStopSwarmRequest);

    /** StopSwarmRequest id. */
    public id: number;

    /**
     * Creates a new StopSwarmRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns StopSwarmRequest instance
     */
    public static create(properties?: IStopSwarmRequest): StopSwarmRequest;

    /**
     * Encodes the specified StopSwarmRequest message. Does not implicitly {@link StopSwarmRequest.verify|verify} messages.
     * @param message StopSwarmRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IStopSwarmRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified StopSwarmRequest message, length delimited. Does not implicitly {@link StopSwarmRequest.verify|verify} messages.
     * @param message StopSwarmRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IStopSwarmRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a StopSwarmRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns StopSwarmRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): StopSwarmRequest;

    /**
     * Decodes a StopSwarmRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns StopSwarmRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): StopSwarmRequest;

    /**
     * Verifies a StopSwarmRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a StopSwarmRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns StopSwarmRequest
     */
    public static fromObject(object: { [k: string]: any }): StopSwarmRequest;

    /**
     * Creates a plain object from a StopSwarmRequest message. Also converts values to other types if specified.
     * @param message StopSwarmRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: StopSwarmRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this StopSwarmRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a StopSwarmResponse. */
export interface IStopSwarmResponse {
}

/** Represents a StopSwarmResponse. */
export class StopSwarmResponse implements IStopSwarmResponse {

    /**
     * Constructs a new StopSwarmResponse.
     * @param [properties] Properties to set
     */
    constructor(properties?: IStopSwarmResponse);

    /**
     * Creates a new StopSwarmResponse instance using the specified properties.
     * @param [properties] Properties to set
     * @returns StopSwarmResponse instance
     */
    public static create(properties?: IStopSwarmResponse): StopSwarmResponse;

    /**
     * Encodes the specified StopSwarmResponse message. Does not implicitly {@link StopSwarmResponse.verify|verify} messages.
     * @param message StopSwarmResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IStopSwarmResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified StopSwarmResponse message, length delimited. Does not implicitly {@link StopSwarmResponse.verify|verify} messages.
     * @param message StopSwarmResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IStopSwarmResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a StopSwarmResponse message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns StopSwarmResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): StopSwarmResponse;

    /**
     * Decodes a StopSwarmResponse message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns StopSwarmResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): StopSwarmResponse;

    /**
     * Verifies a StopSwarmResponse message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a StopSwarmResponse message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns StopSwarmResponse
     */
    public static fromObject(object: { [k: string]: any }): StopSwarmResponse;

    /**
     * Creates a plain object from a StopSwarmResponse message. Also converts values to other types if specified.
     * @param message StopSwarmResponse
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: StopSwarmResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this StopSwarmResponse to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a PublishSwarmRequest. */
export interface IPublishSwarmRequest {

    /** PublishSwarmRequest id */
    id?: (number|null);

    /** PublishSwarmRequest networkKey */
    networkKey?: (Uint8Array|null);
}

/** Represents a PublishSwarmRequest. */
export class PublishSwarmRequest implements IPublishSwarmRequest {

    /**
     * Constructs a new PublishSwarmRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: IPublishSwarmRequest);

    /** PublishSwarmRequest id. */
    public id: number;

    /** PublishSwarmRequest networkKey. */
    public networkKey: Uint8Array;

    /**
     * Creates a new PublishSwarmRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns PublishSwarmRequest instance
     */
    public static create(properties?: IPublishSwarmRequest): PublishSwarmRequest;

    /**
     * Encodes the specified PublishSwarmRequest message. Does not implicitly {@link PublishSwarmRequest.verify|verify} messages.
     * @param message PublishSwarmRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IPublishSwarmRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified PublishSwarmRequest message, length delimited. Does not implicitly {@link PublishSwarmRequest.verify|verify} messages.
     * @param message PublishSwarmRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IPublishSwarmRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a PublishSwarmRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns PublishSwarmRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): PublishSwarmRequest;

    /**
     * Decodes a PublishSwarmRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns PublishSwarmRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): PublishSwarmRequest;

    /**
     * Verifies a PublishSwarmRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a PublishSwarmRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns PublishSwarmRequest
     */
    public static fromObject(object: { [k: string]: any }): PublishSwarmRequest;

    /**
     * Creates a plain object from a PublishSwarmRequest message. Also converts values to other types if specified.
     * @param message PublishSwarmRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: PublishSwarmRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this PublishSwarmRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a PublishSwarmResponse. */
export interface IPublishSwarmResponse {
}

/** Represents a PublishSwarmResponse. */
export class PublishSwarmResponse implements IPublishSwarmResponse {

    /**
     * Constructs a new PublishSwarmResponse.
     * @param [properties] Properties to set
     */
    constructor(properties?: IPublishSwarmResponse);

    /**
     * Creates a new PublishSwarmResponse instance using the specified properties.
     * @param [properties] Properties to set
     * @returns PublishSwarmResponse instance
     */
    public static create(properties?: IPublishSwarmResponse): PublishSwarmResponse;

    /**
     * Encodes the specified PublishSwarmResponse message. Does not implicitly {@link PublishSwarmResponse.verify|verify} messages.
     * @param message PublishSwarmResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IPublishSwarmResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified PublishSwarmResponse message, length delimited. Does not implicitly {@link PublishSwarmResponse.verify|verify} messages.
     * @param message PublishSwarmResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IPublishSwarmResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a PublishSwarmResponse message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns PublishSwarmResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): PublishSwarmResponse;

    /**
     * Decodes a PublishSwarmResponse message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns PublishSwarmResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): PublishSwarmResponse;

    /**
     * Verifies a PublishSwarmResponse message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a PublishSwarmResponse message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns PublishSwarmResponse
     */
    public static fromObject(object: { [k: string]: any }): PublishSwarmResponse;

    /**
     * Creates a plain object from a PublishSwarmResponse message. Also converts values to other types if specified.
     * @param message PublishSwarmResponse
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: PublishSwarmResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this PublishSwarmResponse to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a VideoServerOpenRequest. */
export interface IVideoServerOpenRequest {
}

/** Represents a VideoServerOpenRequest. */
export class VideoServerOpenRequest implements IVideoServerOpenRequest {

    /**
     * Constructs a new VideoServerOpenRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: IVideoServerOpenRequest);

    /**
     * Creates a new VideoServerOpenRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns VideoServerOpenRequest instance
     */
    public static create(properties?: IVideoServerOpenRequest): VideoServerOpenRequest;

    /**
     * Encodes the specified VideoServerOpenRequest message. Does not implicitly {@link VideoServerOpenRequest.verify|verify} messages.
     * @param message VideoServerOpenRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IVideoServerOpenRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified VideoServerOpenRequest message, length delimited. Does not implicitly {@link VideoServerOpenRequest.verify|verify} messages.
     * @param message VideoServerOpenRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IVideoServerOpenRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a VideoServerOpenRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns VideoServerOpenRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): VideoServerOpenRequest;

    /**
     * Decodes a VideoServerOpenRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns VideoServerOpenRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): VideoServerOpenRequest;

    /**
     * Verifies a VideoServerOpenRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a VideoServerOpenRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns VideoServerOpenRequest
     */
    public static fromObject(object: { [k: string]: any }): VideoServerOpenRequest;

    /**
     * Creates a plain object from a VideoServerOpenRequest message. Also converts values to other types if specified.
     * @param message VideoServerOpenRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: VideoServerOpenRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this VideoServerOpenRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a VideoServerOpenResponse. */
export interface IVideoServerOpenResponse {

    /** VideoServerOpenResponse id */
    id?: (number|null);
}

/** Represents a VideoServerOpenResponse. */
export class VideoServerOpenResponse implements IVideoServerOpenResponse {

    /**
     * Constructs a new VideoServerOpenResponse.
     * @param [properties] Properties to set
     */
    constructor(properties?: IVideoServerOpenResponse);

    /** VideoServerOpenResponse id. */
    public id: number;

    /**
     * Creates a new VideoServerOpenResponse instance using the specified properties.
     * @param [properties] Properties to set
     * @returns VideoServerOpenResponse instance
     */
    public static create(properties?: IVideoServerOpenResponse): VideoServerOpenResponse;

    /**
     * Encodes the specified VideoServerOpenResponse message. Does not implicitly {@link VideoServerOpenResponse.verify|verify} messages.
     * @param message VideoServerOpenResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IVideoServerOpenResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified VideoServerOpenResponse message, length delimited. Does not implicitly {@link VideoServerOpenResponse.verify|verify} messages.
     * @param message VideoServerOpenResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IVideoServerOpenResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a VideoServerOpenResponse message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns VideoServerOpenResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): VideoServerOpenResponse;

    /**
     * Decodes a VideoServerOpenResponse message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns VideoServerOpenResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): VideoServerOpenResponse;

    /**
     * Verifies a VideoServerOpenResponse message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a VideoServerOpenResponse message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns VideoServerOpenResponse
     */
    public static fromObject(object: { [k: string]: any }): VideoServerOpenResponse;

    /**
     * Creates a plain object from a VideoServerOpenResponse message. Also converts values to other types if specified.
     * @param message VideoServerOpenResponse
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: VideoServerOpenResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this VideoServerOpenResponse to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a VideoServerWriteRequest. */
export interface IVideoServerWriteRequest {

    /** VideoServerWriteRequest id */
    id?: (number|null);

    /** VideoServerWriteRequest data */
    data?: (Uint8Array|null);

    /** VideoServerWriteRequest flush */
    flush?: (boolean|null);
}

/** Represents a VideoServerWriteRequest. */
export class VideoServerWriteRequest implements IVideoServerWriteRequest {

    /**
     * Constructs a new VideoServerWriteRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: IVideoServerWriteRequest);

    /** VideoServerWriteRequest id. */
    public id: number;

    /** VideoServerWriteRequest data. */
    public data: Uint8Array;

    /** VideoServerWriteRequest flush. */
    public flush: boolean;

    /**
     * Creates a new VideoServerWriteRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns VideoServerWriteRequest instance
     */
    public static create(properties?: IVideoServerWriteRequest): VideoServerWriteRequest;

    /**
     * Encodes the specified VideoServerWriteRequest message. Does not implicitly {@link VideoServerWriteRequest.verify|verify} messages.
     * @param message VideoServerWriteRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IVideoServerWriteRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified VideoServerWriteRequest message, length delimited. Does not implicitly {@link VideoServerWriteRequest.verify|verify} messages.
     * @param message VideoServerWriteRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IVideoServerWriteRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a VideoServerWriteRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns VideoServerWriteRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): VideoServerWriteRequest;

    /**
     * Decodes a VideoServerWriteRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns VideoServerWriteRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): VideoServerWriteRequest;

    /**
     * Verifies a VideoServerWriteRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a VideoServerWriteRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns VideoServerWriteRequest
     */
    public static fromObject(object: { [k: string]: any }): VideoServerWriteRequest;

    /**
     * Creates a plain object from a VideoServerWriteRequest message. Also converts values to other types if specified.
     * @param message VideoServerWriteRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: VideoServerWriteRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this VideoServerWriteRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a VideoServerWriteResponse. */
export interface IVideoServerWriteResponse {
}

/** Represents a VideoServerWriteResponse. */
export class VideoServerWriteResponse implements IVideoServerWriteResponse {

    /**
     * Constructs a new VideoServerWriteResponse.
     * @param [properties] Properties to set
     */
    constructor(properties?: IVideoServerWriteResponse);

    /**
     * Creates a new VideoServerWriteResponse instance using the specified properties.
     * @param [properties] Properties to set
     * @returns VideoServerWriteResponse instance
     */
    public static create(properties?: IVideoServerWriteResponse): VideoServerWriteResponse;

    /**
     * Encodes the specified VideoServerWriteResponse message. Does not implicitly {@link VideoServerWriteResponse.verify|verify} messages.
     * @param message VideoServerWriteResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IVideoServerWriteResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified VideoServerWriteResponse message, length delimited. Does not implicitly {@link VideoServerWriteResponse.verify|verify} messages.
     * @param message VideoServerWriteResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IVideoServerWriteResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a VideoServerWriteResponse message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns VideoServerWriteResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): VideoServerWriteResponse;

    /**
     * Decodes a VideoServerWriteResponse message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns VideoServerWriteResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): VideoServerWriteResponse;

    /**
     * Verifies a VideoServerWriteResponse message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a VideoServerWriteResponse message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns VideoServerWriteResponse
     */
    public static fromObject(object: { [k: string]: any }): VideoServerWriteResponse;

    /**
     * Creates a plain object from a VideoServerWriteResponse message. Also converts values to other types if specified.
     * @param message VideoServerWriteResponse
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: VideoServerWriteResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this VideoServerWriteResponse to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a VideoClientOpenRequest. */
export interface IVideoClientOpenRequest {

    /** VideoClientOpenRequest swarmKey */
    swarmKey?: (Uint8Array|null);

    /** VideoClientOpenRequest emitData */
    emitData?: (boolean|null);
}

/** Represents a VideoClientOpenRequest. */
export class VideoClientOpenRequest implements IVideoClientOpenRequest {

    /**
     * Constructs a new VideoClientOpenRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: IVideoClientOpenRequest);

    /** VideoClientOpenRequest swarmKey. */
    public swarmKey: Uint8Array;

    /** VideoClientOpenRequest emitData. */
    public emitData: boolean;

    /**
     * Creates a new VideoClientOpenRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns VideoClientOpenRequest instance
     */
    public static create(properties?: IVideoClientOpenRequest): VideoClientOpenRequest;

    /**
     * Encodes the specified VideoClientOpenRequest message. Does not implicitly {@link VideoClientOpenRequest.verify|verify} messages.
     * @param message VideoClientOpenRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IVideoClientOpenRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified VideoClientOpenRequest message, length delimited. Does not implicitly {@link VideoClientOpenRequest.verify|verify} messages.
     * @param message VideoClientOpenRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IVideoClientOpenRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a VideoClientOpenRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns VideoClientOpenRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): VideoClientOpenRequest;

    /**
     * Decodes a VideoClientOpenRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns VideoClientOpenRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): VideoClientOpenRequest;

    /**
     * Verifies a VideoClientOpenRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a VideoClientOpenRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns VideoClientOpenRequest
     */
    public static fromObject(object: { [k: string]: any }): VideoClientOpenRequest;

    /**
     * Creates a plain object from a VideoClientOpenRequest message. Also converts values to other types if specified.
     * @param message VideoClientOpenRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: VideoClientOpenRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this VideoClientOpenRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a VideoClientEvent. */
export interface IVideoClientEvent {

    /** VideoClientEvent data */
    data?: (VideoClientEvent.IData|null);

    /** VideoClientEvent open */
    open?: (VideoClientEvent.IOpen|null);

    /** VideoClientEvent close */
    close?: (VideoClientEvent.IClose|null);
}

/** Represents a VideoClientEvent. */
export class VideoClientEvent implements IVideoClientEvent {

    /**
     * Constructs a new VideoClientEvent.
     * @param [properties] Properties to set
     */
    constructor(properties?: IVideoClientEvent);

    /** VideoClientEvent data. */
    public data?: (VideoClientEvent.IData|null);

    /** VideoClientEvent open. */
    public open?: (VideoClientEvent.IOpen|null);

    /** VideoClientEvent close. */
    public close?: (VideoClientEvent.IClose|null);

    /** VideoClientEvent body. */
    public body?: ("data"|"open"|"close");

    /**
     * Creates a new VideoClientEvent instance using the specified properties.
     * @param [properties] Properties to set
     * @returns VideoClientEvent instance
     */
    public static create(properties?: IVideoClientEvent): VideoClientEvent;

    /**
     * Encodes the specified VideoClientEvent message. Does not implicitly {@link VideoClientEvent.verify|verify} messages.
     * @param message VideoClientEvent message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IVideoClientEvent, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified VideoClientEvent message, length delimited. Does not implicitly {@link VideoClientEvent.verify|verify} messages.
     * @param message VideoClientEvent message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IVideoClientEvent, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a VideoClientEvent message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns VideoClientEvent
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): VideoClientEvent;

    /**
     * Decodes a VideoClientEvent message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns VideoClientEvent
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): VideoClientEvent;

    /**
     * Verifies a VideoClientEvent message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a VideoClientEvent message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns VideoClientEvent
     */
    public static fromObject(object: { [k: string]: any }): VideoClientEvent;

    /**
     * Creates a plain object from a VideoClientEvent message. Also converts values to other types if specified.
     * @param message VideoClientEvent
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: VideoClientEvent, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this VideoClientEvent to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

export namespace VideoClientEvent {

    /** Properties of a Data. */
    interface IData {

        /** Data data */
        data?: (Uint8Array|null);

        /** Data flush */
        flush?: (boolean|null);
    }

    /** Represents a Data. */
    class Data implements IData {

        /**
         * Constructs a new Data.
         * @param [properties] Properties to set
         */
        constructor(properties?: VideoClientEvent.IData);

        /** Data data. */
        public data: Uint8Array;

        /** Data flush. */
        public flush: boolean;

        /**
         * Creates a new Data instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Data instance
         */
        public static create(properties?: VideoClientEvent.IData): VideoClientEvent.Data;

        /**
         * Encodes the specified Data message. Does not implicitly {@link VideoClientEvent.Data.verify|verify} messages.
         * @param message Data message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: VideoClientEvent.IData, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Data message, length delimited. Does not implicitly {@link VideoClientEvent.Data.verify|verify} messages.
         * @param message Data message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: VideoClientEvent.IData, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a Data message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Data
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): VideoClientEvent.Data;

        /**
         * Decodes a Data message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Data
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): VideoClientEvent.Data;

        /**
         * Verifies a Data message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a Data message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Data
         */
        public static fromObject(object: { [k: string]: any }): VideoClientEvent.Data;

        /**
         * Creates a plain object from a Data message. Also converts values to other types if specified.
         * @param message Data
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: VideoClientEvent.Data, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Data to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }

    /** Properties of an Open. */
    interface IOpen {

        /** Open id */
        id?: (number|null);
    }

    /** Represents an Open. */
    class Open implements IOpen {

        /**
         * Constructs a new Open.
         * @param [properties] Properties to set
         */
        constructor(properties?: VideoClientEvent.IOpen);

        /** Open id. */
        public id: number;

        /**
         * Creates a new Open instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Open instance
         */
        public static create(properties?: VideoClientEvent.IOpen): VideoClientEvent.Open;

        /**
         * Encodes the specified Open message. Does not implicitly {@link VideoClientEvent.Open.verify|verify} messages.
         * @param message Open message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: VideoClientEvent.IOpen, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Open message, length delimited. Does not implicitly {@link VideoClientEvent.Open.verify|verify} messages.
         * @param message Open message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: VideoClientEvent.IOpen, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes an Open message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Open
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): VideoClientEvent.Open;

        /**
         * Decodes an Open message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Open
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): VideoClientEvent.Open;

        /**
         * Verifies an Open message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates an Open message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Open
         */
        public static fromObject(object: { [k: string]: any }): VideoClientEvent.Open;

        /**
         * Creates a plain object from an Open message. Also converts values to other types if specified.
         * @param message Open
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: VideoClientEvent.Open, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Open to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }

    /** Properties of a Close. */
    interface IClose {
    }

    /** Represents a Close. */
    class Close implements IClose {

        /**
         * Constructs a new Close.
         * @param [properties] Properties to set
         */
        constructor(properties?: VideoClientEvent.IClose);

        /**
         * Creates a new Close instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Close instance
         */
        public static create(properties?: VideoClientEvent.IClose): VideoClientEvent.Close;

        /**
         * Encodes the specified Close message. Does not implicitly {@link VideoClientEvent.Close.verify|verify} messages.
         * @param message Close message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: VideoClientEvent.IClose, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Close message, length delimited. Does not implicitly {@link VideoClientEvent.Close.verify|verify} messages.
         * @param message Close message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: VideoClientEvent.IClose, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a Close message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Close
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): VideoClientEvent.Close;

        /**
         * Decodes a Close message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Close
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): VideoClientEvent.Close;

        /**
         * Verifies a Close message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a Close message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Close
         */
        public static fromObject(object: { [k: string]: any }): VideoClientEvent.Close;

        /**
         * Creates a plain object from a Close message. Also converts values to other types if specified.
         * @param message Close
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: VideoClientEvent.Close, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Close to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }
}

/** Properties of a VideoClientCallRequest. */
export interface IVideoClientCallRequest {

    /** VideoClientCallRequest id */
    id?: (number|null);

    /** VideoClientCallRequest data */
    data?: (VideoClientCallRequest.IData|null);

    /** VideoClientCallRequest runClient */
    runClient?: (VideoClientCallRequest.IRunClient|null);

    /** VideoClientCallRequest runServer */
    runServer?: (VideoClientCallRequest.IRunServer|null);
}

/** Represents a VideoClientCallRequest. */
export class VideoClientCallRequest implements IVideoClientCallRequest {

    /**
     * Constructs a new VideoClientCallRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: IVideoClientCallRequest);

    /** VideoClientCallRequest id. */
    public id: number;

    /** VideoClientCallRequest data. */
    public data?: (VideoClientCallRequest.IData|null);

    /** VideoClientCallRequest runClient. */
    public runClient?: (VideoClientCallRequest.IRunClient|null);

    /** VideoClientCallRequest runServer. */
    public runServer?: (VideoClientCallRequest.IRunServer|null);

    /** VideoClientCallRequest body. */
    public body?: ("data"|"runClient"|"runServer");

    /**
     * Creates a new VideoClientCallRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns VideoClientCallRequest instance
     */
    public static create(properties?: IVideoClientCallRequest): VideoClientCallRequest;

    /**
     * Encodes the specified VideoClientCallRequest message. Does not implicitly {@link VideoClientCallRequest.verify|verify} messages.
     * @param message VideoClientCallRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IVideoClientCallRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified VideoClientCallRequest message, length delimited. Does not implicitly {@link VideoClientCallRequest.verify|verify} messages.
     * @param message VideoClientCallRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IVideoClientCallRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a VideoClientCallRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns VideoClientCallRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): VideoClientCallRequest;

    /**
     * Decodes a VideoClientCallRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns VideoClientCallRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): VideoClientCallRequest;

    /**
     * Verifies a VideoClientCallRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a VideoClientCallRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns VideoClientCallRequest
     */
    public static fromObject(object: { [k: string]: any }): VideoClientCallRequest;

    /**
     * Creates a plain object from a VideoClientCallRequest message. Also converts values to other types if specified.
     * @param message VideoClientCallRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: VideoClientCallRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this VideoClientCallRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

export namespace VideoClientCallRequest {

    /** Properties of a Data. */
    interface IData {

        /** Data body */
        body?: (Uint8Array|null);
    }

    /** Represents a Data. */
    class Data implements IData {

        /**
         * Constructs a new Data.
         * @param [properties] Properties to set
         */
        constructor(properties?: VideoClientCallRequest.IData);

        /** Data body. */
        public body: Uint8Array;

        /**
         * Creates a new Data instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Data instance
         */
        public static create(properties?: VideoClientCallRequest.IData): VideoClientCallRequest.Data;

        /**
         * Encodes the specified Data message. Does not implicitly {@link VideoClientCallRequest.Data.verify|verify} messages.
         * @param message Data message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: VideoClientCallRequest.IData, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Data message, length delimited. Does not implicitly {@link VideoClientCallRequest.Data.verify|verify} messages.
         * @param message Data message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: VideoClientCallRequest.IData, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a Data message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Data
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): VideoClientCallRequest.Data;

        /**
         * Decodes a Data message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Data
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): VideoClientCallRequest.Data;

        /**
         * Verifies a Data message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a Data message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Data
         */
        public static fromObject(object: { [k: string]: any }): VideoClientCallRequest.Data;

        /**
         * Creates a plain object from a Data message. Also converts values to other types if specified.
         * @param message Data
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: VideoClientCallRequest.Data, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Data to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }

    /** Properties of a RunServer. */
    interface IRunServer {
    }

    /** Represents a RunServer. */
    class RunServer implements IRunServer {

        /**
         * Constructs a new RunServer.
         * @param [properties] Properties to set
         */
        constructor(properties?: VideoClientCallRequest.IRunServer);

        /**
         * Creates a new RunServer instance using the specified properties.
         * @param [properties] Properties to set
         * @returns RunServer instance
         */
        public static create(properties?: VideoClientCallRequest.IRunServer): VideoClientCallRequest.RunServer;

        /**
         * Encodes the specified RunServer message. Does not implicitly {@link VideoClientCallRequest.RunServer.verify|verify} messages.
         * @param message RunServer message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: VideoClientCallRequest.IRunServer, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified RunServer message, length delimited. Does not implicitly {@link VideoClientCallRequest.RunServer.verify|verify} messages.
         * @param message RunServer message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: VideoClientCallRequest.IRunServer, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a RunServer message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns RunServer
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): VideoClientCallRequest.RunServer;

        /**
         * Decodes a RunServer message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns RunServer
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): VideoClientCallRequest.RunServer;

        /**
         * Verifies a RunServer message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a RunServer message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns RunServer
         */
        public static fromObject(object: { [k: string]: any }): VideoClientCallRequest.RunServer;

        /**
         * Creates a plain object from a RunServer message. Also converts values to other types if specified.
         * @param message RunServer
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: VideoClientCallRequest.RunServer, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this RunServer to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }

    /** Properties of a RunClient. */
    interface IRunClient {
    }

    /** Represents a RunClient. */
    class RunClient implements IRunClient {

        /**
         * Constructs a new RunClient.
         * @param [properties] Properties to set
         */
        constructor(properties?: VideoClientCallRequest.IRunClient);

        /**
         * Creates a new RunClient instance using the specified properties.
         * @param [properties] Properties to set
         * @returns RunClient instance
         */
        public static create(properties?: VideoClientCallRequest.IRunClient): VideoClientCallRequest.RunClient;

        /**
         * Encodes the specified RunClient message. Does not implicitly {@link VideoClientCallRequest.RunClient.verify|verify} messages.
         * @param message RunClient message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: VideoClientCallRequest.IRunClient, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified RunClient message, length delimited. Does not implicitly {@link VideoClientCallRequest.RunClient.verify|verify} messages.
         * @param message RunClient message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: VideoClientCallRequest.IRunClient, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a RunClient message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns RunClient
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): VideoClientCallRequest.RunClient;

        /**
         * Decodes a RunClient message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns RunClient
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): VideoClientCallRequest.RunClient;

        /**
         * Verifies a RunClient message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a RunClient message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns RunClient
         */
        public static fromObject(object: { [k: string]: any }): VideoClientCallRequest.RunClient;

        /**
         * Creates a plain object from a RunClient message. Also converts values to other types if specified.
         * @param message RunClient
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: VideoClientCallRequest.RunClient, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this RunClient to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }
}

/** Properties of a SwarmThingMessage. */
export interface ISwarmThingMessage {

    /** SwarmThingMessage open */
    open?: (SwarmThingMessage.IOpen|null);

    /** SwarmThingMessage close */
    close?: (SwarmThingMessage.IClose|null);
}

/** Represents a SwarmThingMessage. */
export class SwarmThingMessage implements ISwarmThingMessage {

    /**
     * Constructs a new SwarmThingMessage.
     * @param [properties] Properties to set
     */
    constructor(properties?: ISwarmThingMessage);

    /** SwarmThingMessage open. */
    public open?: (SwarmThingMessage.IOpen|null);

    /** SwarmThingMessage close. */
    public close?: (SwarmThingMessage.IClose|null);

    /** SwarmThingMessage body. */
    public body?: ("open"|"close");

    /**
     * Creates a new SwarmThingMessage instance using the specified properties.
     * @param [properties] Properties to set
     * @returns SwarmThingMessage instance
     */
    public static create(properties?: ISwarmThingMessage): SwarmThingMessage;

    /**
     * Encodes the specified SwarmThingMessage message. Does not implicitly {@link SwarmThingMessage.verify|verify} messages.
     * @param message SwarmThingMessage message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: ISwarmThingMessage, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified SwarmThingMessage message, length delimited. Does not implicitly {@link SwarmThingMessage.verify|verify} messages.
     * @param message SwarmThingMessage message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: ISwarmThingMessage, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a SwarmThingMessage message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns SwarmThingMessage
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): SwarmThingMessage;

    /**
     * Decodes a SwarmThingMessage message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns SwarmThingMessage
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): SwarmThingMessage;

    /**
     * Verifies a SwarmThingMessage message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a SwarmThingMessage message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns SwarmThingMessage
     */
    public static fromObject(object: { [k: string]: any }): SwarmThingMessage;

    /**
     * Creates a plain object from a SwarmThingMessage message. Also converts values to other types if specified.
     * @param message SwarmThingMessage
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: SwarmThingMessage, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this SwarmThingMessage to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

export namespace SwarmThingMessage {

    /** Properties of an Open. */
    interface IOpen {

        /** Open swarmId */
        swarmId?: (Uint8Array|null);

        /** Open port */
        port?: (number|null);
    }

    /** Represents an Open. */
    class Open implements IOpen {

        /**
         * Constructs a new Open.
         * @param [properties] Properties to set
         */
        constructor(properties?: SwarmThingMessage.IOpen);

        /** Open swarmId. */
        public swarmId: Uint8Array;

        /** Open port. */
        public port: number;

        /**
         * Creates a new Open instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Open instance
         */
        public static create(properties?: SwarmThingMessage.IOpen): SwarmThingMessage.Open;

        /**
         * Encodes the specified Open message. Does not implicitly {@link SwarmThingMessage.Open.verify|verify} messages.
         * @param message Open message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: SwarmThingMessage.IOpen, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Open message, length delimited. Does not implicitly {@link SwarmThingMessage.Open.verify|verify} messages.
         * @param message Open message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: SwarmThingMessage.IOpen, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes an Open message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Open
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): SwarmThingMessage.Open;

        /**
         * Decodes an Open message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Open
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): SwarmThingMessage.Open;

        /**
         * Verifies an Open message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates an Open message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Open
         */
        public static fromObject(object: { [k: string]: any }): SwarmThingMessage.Open;

        /**
         * Creates a plain object from an Open message. Also converts values to other types if specified.
         * @param message Open
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: SwarmThingMessage.Open, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Open to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }

    /** Properties of a Close. */
    interface IClose {

        /** Close swarmId */
        swarmId?: (Uint8Array|null);
    }

    /** Represents a Close. */
    class Close implements IClose {

        /**
         * Constructs a new Close.
         * @param [properties] Properties to set
         */
        constructor(properties?: SwarmThingMessage.IClose);

        /** Close swarmId. */
        public swarmId: Uint8Array;

        /**
         * Creates a new Close instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Close instance
         */
        public static create(properties?: SwarmThingMessage.IClose): SwarmThingMessage.Close;

        /**
         * Encodes the specified Close message. Does not implicitly {@link SwarmThingMessage.Close.verify|verify} messages.
         * @param message Close message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: SwarmThingMessage.IClose, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Close message, length delimited. Does not implicitly {@link SwarmThingMessage.Close.verify|verify} messages.
         * @param message Close message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: SwarmThingMessage.IClose, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a Close message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Close
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): SwarmThingMessage.Close;

        /**
         * Decodes a Close message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Close
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): SwarmThingMessage.Close;

        /**
         * Verifies a Close message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a Close message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Close
         */
        public static fromObject(object: { [k: string]: any }): SwarmThingMessage.Close;

        /**
         * Creates a plain object from a Close message. Also converts values to other types if specified.
         * @param message Close
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: SwarmThingMessage.Close, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Close to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }
}

/** Properties of a PeerIndexMessage. */
export interface IPeerIndexMessage {

    /** PeerIndexMessage publish */
    publish?: (PeerIndexMessage.IPublish|null);

    /** PeerIndexMessage unpublish */
    unpublish?: (PeerIndexMessage.IUnpublish|null);

    /** PeerIndexMessage searchRequest */
    searchRequest?: (PeerIndexMessage.ISearchRequest|null);

    /** PeerIndexMessage searchResponse */
    searchResponse?: (PeerIndexMessage.ISearchResponse|null);
}

/** Represents a PeerIndexMessage. */
export class PeerIndexMessage implements IPeerIndexMessage {

    /**
     * Constructs a new PeerIndexMessage.
     * @param [properties] Properties to set
     */
    constructor(properties?: IPeerIndexMessage);

    /** PeerIndexMessage publish. */
    public publish?: (PeerIndexMessage.IPublish|null);

    /** PeerIndexMessage unpublish. */
    public unpublish?: (PeerIndexMessage.IUnpublish|null);

    /** PeerIndexMessage searchRequest. */
    public searchRequest?: (PeerIndexMessage.ISearchRequest|null);

    /** PeerIndexMessage searchResponse. */
    public searchResponse?: (PeerIndexMessage.ISearchResponse|null);

    /** PeerIndexMessage body. */
    public body?: ("publish"|"unpublish"|"searchRequest"|"searchResponse");

    /**
     * Creates a new PeerIndexMessage instance using the specified properties.
     * @param [properties] Properties to set
     * @returns PeerIndexMessage instance
     */
    public static create(properties?: IPeerIndexMessage): PeerIndexMessage;

    /**
     * Encodes the specified PeerIndexMessage message. Does not implicitly {@link PeerIndexMessage.verify|verify} messages.
     * @param message PeerIndexMessage message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IPeerIndexMessage, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified PeerIndexMessage message, length delimited. Does not implicitly {@link PeerIndexMessage.verify|verify} messages.
     * @param message PeerIndexMessage message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IPeerIndexMessage, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a PeerIndexMessage message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns PeerIndexMessage
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): PeerIndexMessage;

    /**
     * Decodes a PeerIndexMessage message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns PeerIndexMessage
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): PeerIndexMessage;

    /**
     * Verifies a PeerIndexMessage message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a PeerIndexMessage message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns PeerIndexMessage
     */
    public static fromObject(object: { [k: string]: any }): PeerIndexMessage;

    /**
     * Creates a plain object from a PeerIndexMessage message. Also converts values to other types if specified.
     * @param message PeerIndexMessage
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: PeerIndexMessage, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this PeerIndexMessage to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

export namespace PeerIndexMessage {

    /** Properties of a Record. */
    interface IRecord {

        /** Record hash */
        hash?: (Uint8Array|null);

        /** Record key */
        key?: (Uint8Array|null);

        /** Record hostId */
        hostId?: (Uint8Array|null);

        /** Record port */
        port?: (number|null);

        /** Record timestamp */
        timestamp?: (number|null);

        /** Record signature */
        signature?: (Uint8Array|null);
    }

    /** Represents a Record. */
    class Record implements IRecord {

        /**
         * Constructs a new Record.
         * @param [properties] Properties to set
         */
        constructor(properties?: PeerIndexMessage.IRecord);

        /** Record hash. */
        public hash: Uint8Array;

        /** Record key. */
        public key: Uint8Array;

        /** Record hostId. */
        public hostId: Uint8Array;

        /** Record port. */
        public port: number;

        /** Record timestamp. */
        public timestamp: number;

        /** Record signature. */
        public signature: Uint8Array;

        /**
         * Creates a new Record instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Record instance
         */
        public static create(properties?: PeerIndexMessage.IRecord): PeerIndexMessage.Record;

        /**
         * Encodes the specified Record message. Does not implicitly {@link PeerIndexMessage.Record.verify|verify} messages.
         * @param message Record message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: PeerIndexMessage.IRecord, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Record message, length delimited. Does not implicitly {@link PeerIndexMessage.Record.verify|verify} messages.
         * @param message Record message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: PeerIndexMessage.IRecord, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a Record message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Record
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): PeerIndexMessage.Record;

        /**
         * Decodes a Record message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Record
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): PeerIndexMessage.Record;

        /**
         * Verifies a Record message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a Record message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Record
         */
        public static fromObject(object: { [k: string]: any }): PeerIndexMessage.Record;

        /**
         * Creates a plain object from a Record message. Also converts values to other types if specified.
         * @param message Record
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: PeerIndexMessage.Record, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Record to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }

    /** Properties of a Publish. */
    interface IPublish {

        /** Publish record */
        record?: (PeerIndexMessage.IRecord|null);
    }

    /** Represents a Publish. */
    class Publish implements IPublish {

        /**
         * Constructs a new Publish.
         * @param [properties] Properties to set
         */
        constructor(properties?: PeerIndexMessage.IPublish);

        /** Publish record. */
        public record?: (PeerIndexMessage.IRecord|null);

        /**
         * Creates a new Publish instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Publish instance
         */
        public static create(properties?: PeerIndexMessage.IPublish): PeerIndexMessage.Publish;

        /**
         * Encodes the specified Publish message. Does not implicitly {@link PeerIndexMessage.Publish.verify|verify} messages.
         * @param message Publish message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: PeerIndexMessage.IPublish, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Publish message, length delimited. Does not implicitly {@link PeerIndexMessage.Publish.verify|verify} messages.
         * @param message Publish message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: PeerIndexMessage.IPublish, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a Publish message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Publish
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): PeerIndexMessage.Publish;

        /**
         * Decodes a Publish message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Publish
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): PeerIndexMessage.Publish;

        /**
         * Verifies a Publish message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a Publish message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Publish
         */
        public static fromObject(object: { [k: string]: any }): PeerIndexMessage.Publish;

        /**
         * Creates a plain object from a Publish message. Also converts values to other types if specified.
         * @param message Publish
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: PeerIndexMessage.Publish, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Publish to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }

    /** Properties of an Unpublish. */
    interface IUnpublish {

        /** Unpublish record */
        record?: (PeerIndexMessage.IRecord|null);
    }

    /** Represents an Unpublish. */
    class Unpublish implements IUnpublish {

        /**
         * Constructs a new Unpublish.
         * @param [properties] Properties to set
         */
        constructor(properties?: PeerIndexMessage.IUnpublish);

        /** Unpublish record. */
        public record?: (PeerIndexMessage.IRecord|null);

        /**
         * Creates a new Unpublish instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Unpublish instance
         */
        public static create(properties?: PeerIndexMessage.IUnpublish): PeerIndexMessage.Unpublish;

        /**
         * Encodes the specified Unpublish message. Does not implicitly {@link PeerIndexMessage.Unpublish.verify|verify} messages.
         * @param message Unpublish message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: PeerIndexMessage.IUnpublish, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Unpublish message, length delimited. Does not implicitly {@link PeerIndexMessage.Unpublish.verify|verify} messages.
         * @param message Unpublish message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: PeerIndexMessage.IUnpublish, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes an Unpublish message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Unpublish
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): PeerIndexMessage.Unpublish;

        /**
         * Decodes an Unpublish message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Unpublish
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): PeerIndexMessage.Unpublish;

        /**
         * Verifies an Unpublish message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates an Unpublish message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Unpublish
         */
        public static fromObject(object: { [k: string]: any }): PeerIndexMessage.Unpublish;

        /**
         * Creates a plain object from an Unpublish message. Also converts values to other types if specified.
         * @param message Unpublish
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: PeerIndexMessage.Unpublish, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Unpublish to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }

    /** Properties of a SearchRequest. */
    interface ISearchRequest {

        /** SearchRequest requestId */
        requestId?: (number|null);

        /** SearchRequest hash */
        hash?: (Uint8Array|null);
    }

    /** Represents a SearchRequest. */
    class SearchRequest implements ISearchRequest {

        /**
         * Constructs a new SearchRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: PeerIndexMessage.ISearchRequest);

        /** SearchRequest requestId. */
        public requestId: number;

        /** SearchRequest hash. */
        public hash: Uint8Array;

        /**
         * Creates a new SearchRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns SearchRequest instance
         */
        public static create(properties?: PeerIndexMessage.ISearchRequest): PeerIndexMessage.SearchRequest;

        /**
         * Encodes the specified SearchRequest message. Does not implicitly {@link PeerIndexMessage.SearchRequest.verify|verify} messages.
         * @param message SearchRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: PeerIndexMessage.ISearchRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified SearchRequest message, length delimited. Does not implicitly {@link PeerIndexMessage.SearchRequest.verify|verify} messages.
         * @param message SearchRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: PeerIndexMessage.ISearchRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a SearchRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns SearchRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): PeerIndexMessage.SearchRequest;

        /**
         * Decodes a SearchRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns SearchRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): PeerIndexMessage.SearchRequest;

        /**
         * Verifies a SearchRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a SearchRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns SearchRequest
         */
        public static fromObject(object: { [k: string]: any }): PeerIndexMessage.SearchRequest;

        /**
         * Creates a plain object from a SearchRequest message. Also converts values to other types if specified.
         * @param message SearchRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: PeerIndexMessage.SearchRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this SearchRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }

    /** Properties of a SearchResponse. */
    interface ISearchResponse {

        /** SearchResponse requestId */
        requestId?: (number|null);

        /** SearchResponse records */
        records?: (PeerIndexMessage.IRecord[]|null);
    }

    /** Represents a SearchResponse. */
    class SearchResponse implements ISearchResponse {

        /**
         * Constructs a new SearchResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: PeerIndexMessage.ISearchResponse);

        /** SearchResponse requestId. */
        public requestId: number;

        /** SearchResponse records. */
        public records: PeerIndexMessage.IRecord[];

        /**
         * Creates a new SearchResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns SearchResponse instance
         */
        public static create(properties?: PeerIndexMessage.ISearchResponse): PeerIndexMessage.SearchResponse;

        /**
         * Encodes the specified SearchResponse message. Does not implicitly {@link PeerIndexMessage.SearchResponse.verify|verify} messages.
         * @param message SearchResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: PeerIndexMessage.ISearchResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified SearchResponse message, length delimited. Does not implicitly {@link PeerIndexMessage.SearchResponse.verify|verify} messages.
         * @param message SearchResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: PeerIndexMessage.ISearchResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a SearchResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns SearchResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): PeerIndexMessage.SearchResponse;

        /**
         * Decodes a SearchResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns SearchResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): PeerIndexMessage.SearchResponse;

        /**
         * Verifies a SearchResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a SearchResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns SearchResponse
         */
        public static fromObject(object: { [k: string]: any }): PeerIndexMessage.SearchResponse;

        /**
         * Creates a plain object from a SearchResponse message. Also converts values to other types if specified.
         * @param message SearchResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: PeerIndexMessage.SearchResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this SearchResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }
}

/** Properties of a CreateChatServerRequest. */
export interface ICreateChatServerRequest {

    /** CreateChatServerRequest networkKey */
    networkKey?: (Uint8Array|null);

    /** CreateChatServerRequest chatRoom */
    chatRoom?: (IChatRoom|null);
}

/** Represents a CreateChatServerRequest. */
export class CreateChatServerRequest implements ICreateChatServerRequest {

    /**
     * Constructs a new CreateChatServerRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: ICreateChatServerRequest);

    /** CreateChatServerRequest networkKey. */
    public networkKey: Uint8Array;

    /** CreateChatServerRequest chatRoom. */
    public chatRoom?: (IChatRoom|null);

    /**
     * Creates a new CreateChatServerRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns CreateChatServerRequest instance
     */
    public static create(properties?: ICreateChatServerRequest): CreateChatServerRequest;

    /**
     * Encodes the specified CreateChatServerRequest message. Does not implicitly {@link CreateChatServerRequest.verify|verify} messages.
     * @param message CreateChatServerRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: ICreateChatServerRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified CreateChatServerRequest message, length delimited. Does not implicitly {@link CreateChatServerRequest.verify|verify} messages.
     * @param message CreateChatServerRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: ICreateChatServerRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a CreateChatServerRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns CreateChatServerRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): CreateChatServerRequest;

    /**
     * Decodes a CreateChatServerRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns CreateChatServerRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): CreateChatServerRequest;

    /**
     * Verifies a CreateChatServerRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a CreateChatServerRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns CreateChatServerRequest
     */
    public static fromObject(object: { [k: string]: any }): CreateChatServerRequest;

    /**
     * Creates a plain object from a CreateChatServerRequest message. Also converts values to other types if specified.
     * @param message CreateChatServerRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: CreateChatServerRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this CreateChatServerRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a CreateChatServerResponse. */
export interface ICreateChatServerResponse {

    /** CreateChatServerResponse chatServer */
    chatServer?: (IChatServer|null);
}

/** Represents a CreateChatServerResponse. */
export class CreateChatServerResponse implements ICreateChatServerResponse {

    /**
     * Constructs a new CreateChatServerResponse.
     * @param [properties] Properties to set
     */
    constructor(properties?: ICreateChatServerResponse);

    /** CreateChatServerResponse chatServer. */
    public chatServer?: (IChatServer|null);

    /**
     * Creates a new CreateChatServerResponse instance using the specified properties.
     * @param [properties] Properties to set
     * @returns CreateChatServerResponse instance
     */
    public static create(properties?: ICreateChatServerResponse): CreateChatServerResponse;

    /**
     * Encodes the specified CreateChatServerResponse message. Does not implicitly {@link CreateChatServerResponse.verify|verify} messages.
     * @param message CreateChatServerResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: ICreateChatServerResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified CreateChatServerResponse message, length delimited. Does not implicitly {@link CreateChatServerResponse.verify|verify} messages.
     * @param message CreateChatServerResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: ICreateChatServerResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a CreateChatServerResponse message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns CreateChatServerResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): CreateChatServerResponse;

    /**
     * Decodes a CreateChatServerResponse message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns CreateChatServerResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): CreateChatServerResponse;

    /**
     * Verifies a CreateChatServerResponse message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a CreateChatServerResponse message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns CreateChatServerResponse
     */
    public static fromObject(object: { [k: string]: any }): CreateChatServerResponse;

    /**
     * Creates a plain object from a CreateChatServerResponse message. Also converts values to other types if specified.
     * @param message CreateChatServerResponse
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: CreateChatServerResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this CreateChatServerResponse to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of an UpdateChatServerRequest. */
export interface IUpdateChatServerRequest {

    /** UpdateChatServerRequest id */
    id?: (number|null);

    /** UpdateChatServerRequest networkKey */
    networkKey?: (Uint8Array|null);

    /** UpdateChatServerRequest serverKey */
    serverKey?: (IChatRoom|null);
}

/** Represents an UpdateChatServerRequest. */
export class UpdateChatServerRequest implements IUpdateChatServerRequest {

    /**
     * Constructs a new UpdateChatServerRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: IUpdateChatServerRequest);

    /** UpdateChatServerRequest id. */
    public id: number;

    /** UpdateChatServerRequest networkKey. */
    public networkKey: Uint8Array;

    /** UpdateChatServerRequest serverKey. */
    public serverKey?: (IChatRoom|null);

    /**
     * Creates a new UpdateChatServerRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns UpdateChatServerRequest instance
     */
    public static create(properties?: IUpdateChatServerRequest): UpdateChatServerRequest;

    /**
     * Encodes the specified UpdateChatServerRequest message. Does not implicitly {@link UpdateChatServerRequest.verify|verify} messages.
     * @param message UpdateChatServerRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IUpdateChatServerRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified UpdateChatServerRequest message, length delimited. Does not implicitly {@link UpdateChatServerRequest.verify|verify} messages.
     * @param message UpdateChatServerRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IUpdateChatServerRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes an UpdateChatServerRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns UpdateChatServerRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): UpdateChatServerRequest;

    /**
     * Decodes an UpdateChatServerRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns UpdateChatServerRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): UpdateChatServerRequest;

    /**
     * Verifies an UpdateChatServerRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates an UpdateChatServerRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns UpdateChatServerRequest
     */
    public static fromObject(object: { [k: string]: any }): UpdateChatServerRequest;

    /**
     * Creates a plain object from an UpdateChatServerRequest message. Also converts values to other types if specified.
     * @param message UpdateChatServerRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: UpdateChatServerRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this UpdateChatServerRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of an UpdateChatServerResponse. */
export interface IUpdateChatServerResponse {

    /** UpdateChatServerResponse chatServer */
    chatServer?: (IChatServer|null);
}

/** Represents an UpdateChatServerResponse. */
export class UpdateChatServerResponse implements IUpdateChatServerResponse {

    /**
     * Constructs a new UpdateChatServerResponse.
     * @param [properties] Properties to set
     */
    constructor(properties?: IUpdateChatServerResponse);

    /** UpdateChatServerResponse chatServer. */
    public chatServer?: (IChatServer|null);

    /**
     * Creates a new UpdateChatServerResponse instance using the specified properties.
     * @param [properties] Properties to set
     * @returns UpdateChatServerResponse instance
     */
    public static create(properties?: IUpdateChatServerResponse): UpdateChatServerResponse;

    /**
     * Encodes the specified UpdateChatServerResponse message. Does not implicitly {@link UpdateChatServerResponse.verify|verify} messages.
     * @param message UpdateChatServerResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IUpdateChatServerResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified UpdateChatServerResponse message, length delimited. Does not implicitly {@link UpdateChatServerResponse.verify|verify} messages.
     * @param message UpdateChatServerResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IUpdateChatServerResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes an UpdateChatServerResponse message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns UpdateChatServerResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): UpdateChatServerResponse;

    /**
     * Decodes an UpdateChatServerResponse message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns UpdateChatServerResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): UpdateChatServerResponse;

    /**
     * Verifies an UpdateChatServerResponse message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates an UpdateChatServerResponse message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns UpdateChatServerResponse
     */
    public static fromObject(object: { [k: string]: any }): UpdateChatServerResponse;

    /**
     * Creates a plain object from an UpdateChatServerResponse message. Also converts values to other types if specified.
     * @param message UpdateChatServerResponse
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: UpdateChatServerResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this UpdateChatServerResponse to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a DeleteChatServerRequest. */
export interface IDeleteChatServerRequest {

    /** DeleteChatServerRequest id */
    id?: (number|null);
}

/** Represents a DeleteChatServerRequest. */
export class DeleteChatServerRequest implements IDeleteChatServerRequest {

    /**
     * Constructs a new DeleteChatServerRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: IDeleteChatServerRequest);

    /** DeleteChatServerRequest id. */
    public id: number;

    /**
     * Creates a new DeleteChatServerRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns DeleteChatServerRequest instance
     */
    public static create(properties?: IDeleteChatServerRequest): DeleteChatServerRequest;

    /**
     * Encodes the specified DeleteChatServerRequest message. Does not implicitly {@link DeleteChatServerRequest.verify|verify} messages.
     * @param message DeleteChatServerRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IDeleteChatServerRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified DeleteChatServerRequest message, length delimited. Does not implicitly {@link DeleteChatServerRequest.verify|verify} messages.
     * @param message DeleteChatServerRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IDeleteChatServerRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a DeleteChatServerRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns DeleteChatServerRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): DeleteChatServerRequest;

    /**
     * Decodes a DeleteChatServerRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns DeleteChatServerRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): DeleteChatServerRequest;

    /**
     * Verifies a DeleteChatServerRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a DeleteChatServerRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns DeleteChatServerRequest
     */
    public static fromObject(object: { [k: string]: any }): DeleteChatServerRequest;

    /**
     * Creates a plain object from a DeleteChatServerRequest message. Also converts values to other types if specified.
     * @param message DeleteChatServerRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: DeleteChatServerRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this DeleteChatServerRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a DeleteChatServerResponse. */
export interface IDeleteChatServerResponse {
}

/** Represents a DeleteChatServerResponse. */
export class DeleteChatServerResponse implements IDeleteChatServerResponse {

    /**
     * Constructs a new DeleteChatServerResponse.
     * @param [properties] Properties to set
     */
    constructor(properties?: IDeleteChatServerResponse);

    /**
     * Creates a new DeleteChatServerResponse instance using the specified properties.
     * @param [properties] Properties to set
     * @returns DeleteChatServerResponse instance
     */
    public static create(properties?: IDeleteChatServerResponse): DeleteChatServerResponse;

    /**
     * Encodes the specified DeleteChatServerResponse message. Does not implicitly {@link DeleteChatServerResponse.verify|verify} messages.
     * @param message DeleteChatServerResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IDeleteChatServerResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified DeleteChatServerResponse message, length delimited. Does not implicitly {@link DeleteChatServerResponse.verify|verify} messages.
     * @param message DeleteChatServerResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IDeleteChatServerResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a DeleteChatServerResponse message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns DeleteChatServerResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): DeleteChatServerResponse;

    /**
     * Decodes a DeleteChatServerResponse message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns DeleteChatServerResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): DeleteChatServerResponse;

    /**
     * Verifies a DeleteChatServerResponse message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a DeleteChatServerResponse message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns DeleteChatServerResponse
     */
    public static fromObject(object: { [k: string]: any }): DeleteChatServerResponse;

    /**
     * Creates a plain object from a DeleteChatServerResponse message. Also converts values to other types if specified.
     * @param message DeleteChatServerResponse
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: DeleteChatServerResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this DeleteChatServerResponse to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a GetChatServerRequest. */
export interface IGetChatServerRequest {

    /** GetChatServerRequest id */
    id?: (number|null);
}

/** Represents a GetChatServerRequest. */
export class GetChatServerRequest implements IGetChatServerRequest {

    /**
     * Constructs a new GetChatServerRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: IGetChatServerRequest);

    /** GetChatServerRequest id. */
    public id: number;

    /**
     * Creates a new GetChatServerRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns GetChatServerRequest instance
     */
    public static create(properties?: IGetChatServerRequest): GetChatServerRequest;

    /**
     * Encodes the specified GetChatServerRequest message. Does not implicitly {@link GetChatServerRequest.verify|verify} messages.
     * @param message GetChatServerRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IGetChatServerRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified GetChatServerRequest message, length delimited. Does not implicitly {@link GetChatServerRequest.verify|verify} messages.
     * @param message GetChatServerRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IGetChatServerRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a GetChatServerRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns GetChatServerRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): GetChatServerRequest;

    /**
     * Decodes a GetChatServerRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns GetChatServerRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): GetChatServerRequest;

    /**
     * Verifies a GetChatServerRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a GetChatServerRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns GetChatServerRequest
     */
    public static fromObject(object: { [k: string]: any }): GetChatServerRequest;

    /**
     * Creates a plain object from a GetChatServerRequest message. Also converts values to other types if specified.
     * @param message GetChatServerRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: GetChatServerRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this GetChatServerRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a GetChatServerResponse. */
export interface IGetChatServerResponse {

    /** GetChatServerResponse chatServer */
    chatServer?: (IChatServer|null);
}

/** Represents a GetChatServerResponse. */
export class GetChatServerResponse implements IGetChatServerResponse {

    /**
     * Constructs a new GetChatServerResponse.
     * @param [properties] Properties to set
     */
    constructor(properties?: IGetChatServerResponse);

    /** GetChatServerResponse chatServer. */
    public chatServer?: (IChatServer|null);

    /**
     * Creates a new GetChatServerResponse instance using the specified properties.
     * @param [properties] Properties to set
     * @returns GetChatServerResponse instance
     */
    public static create(properties?: IGetChatServerResponse): GetChatServerResponse;

    /**
     * Encodes the specified GetChatServerResponse message. Does not implicitly {@link GetChatServerResponse.verify|verify} messages.
     * @param message GetChatServerResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IGetChatServerResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified GetChatServerResponse message, length delimited. Does not implicitly {@link GetChatServerResponse.verify|verify} messages.
     * @param message GetChatServerResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IGetChatServerResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a GetChatServerResponse message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns GetChatServerResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): GetChatServerResponse;

    /**
     * Decodes a GetChatServerResponse message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns GetChatServerResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): GetChatServerResponse;

    /**
     * Verifies a GetChatServerResponse message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a GetChatServerResponse message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns GetChatServerResponse
     */
    public static fromObject(object: { [k: string]: any }): GetChatServerResponse;

    /**
     * Creates a plain object from a GetChatServerResponse message. Also converts values to other types if specified.
     * @param message GetChatServerResponse
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: GetChatServerResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this GetChatServerResponse to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a GetChatServersRequest. */
export interface IGetChatServersRequest {
}

/** Represents a GetChatServersRequest. */
export class GetChatServersRequest implements IGetChatServersRequest {

    /**
     * Constructs a new GetChatServersRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: IGetChatServersRequest);

    /**
     * Creates a new GetChatServersRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns GetChatServersRequest instance
     */
    public static create(properties?: IGetChatServersRequest): GetChatServersRequest;

    /**
     * Encodes the specified GetChatServersRequest message. Does not implicitly {@link GetChatServersRequest.verify|verify} messages.
     * @param message GetChatServersRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IGetChatServersRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified GetChatServersRequest message, length delimited. Does not implicitly {@link GetChatServersRequest.verify|verify} messages.
     * @param message GetChatServersRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IGetChatServersRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a GetChatServersRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns GetChatServersRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): GetChatServersRequest;

    /**
     * Decodes a GetChatServersRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns GetChatServersRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): GetChatServersRequest;

    /**
     * Verifies a GetChatServersRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a GetChatServersRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns GetChatServersRequest
     */
    public static fromObject(object: { [k: string]: any }): GetChatServersRequest;

    /**
     * Creates a plain object from a GetChatServersRequest message. Also converts values to other types if specified.
     * @param message GetChatServersRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: GetChatServersRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this GetChatServersRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a GetChatServersResponse. */
export interface IGetChatServersResponse {

    /** GetChatServersResponse chatServers */
    chatServers?: (IChatServer[]|null);
}

/** Represents a GetChatServersResponse. */
export class GetChatServersResponse implements IGetChatServersResponse {

    /**
     * Constructs a new GetChatServersResponse.
     * @param [properties] Properties to set
     */
    constructor(properties?: IGetChatServersResponse);

    /** GetChatServersResponse chatServers. */
    public chatServers: IChatServer[];

    /**
     * Creates a new GetChatServersResponse instance using the specified properties.
     * @param [properties] Properties to set
     * @returns GetChatServersResponse instance
     */
    public static create(properties?: IGetChatServersResponse): GetChatServersResponse;

    /**
     * Encodes the specified GetChatServersResponse message. Does not implicitly {@link GetChatServersResponse.verify|verify} messages.
     * @param message GetChatServersResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IGetChatServersResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified GetChatServersResponse message, length delimited. Does not implicitly {@link GetChatServersResponse.verify|verify} messages.
     * @param message GetChatServersResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IGetChatServersResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a GetChatServersResponse message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns GetChatServersResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): GetChatServersResponse;

    /**
     * Decodes a GetChatServersResponse message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns GetChatServersResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): GetChatServersResponse;

    /**
     * Verifies a GetChatServersResponse message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a GetChatServersResponse message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns GetChatServersResponse
     */
    public static fromObject(object: { [k: string]: any }): GetChatServersResponse;

    /**
     * Creates a plain object from a GetChatServersResponse message. Also converts values to other types if specified.
     * @param message GetChatServersResponse
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: GetChatServersResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this GetChatServersResponse to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of an OpenChatServerRequest. */
export interface IOpenChatServerRequest {

    /** OpenChatServerRequest server */
    server?: (IChatServer|null);
}

/** Represents an OpenChatServerRequest. */
export class OpenChatServerRequest implements IOpenChatServerRequest {

    /**
     * Constructs a new OpenChatServerRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: IOpenChatServerRequest);

    /** OpenChatServerRequest server. */
    public server?: (IChatServer|null);

    /**
     * Creates a new OpenChatServerRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns OpenChatServerRequest instance
     */
    public static create(properties?: IOpenChatServerRequest): OpenChatServerRequest;

    /**
     * Encodes the specified OpenChatServerRequest message. Does not implicitly {@link OpenChatServerRequest.verify|verify} messages.
     * @param message OpenChatServerRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IOpenChatServerRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified OpenChatServerRequest message, length delimited. Does not implicitly {@link OpenChatServerRequest.verify|verify} messages.
     * @param message OpenChatServerRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IOpenChatServerRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes an OpenChatServerRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns OpenChatServerRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): OpenChatServerRequest;

    /**
     * Decodes an OpenChatServerRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns OpenChatServerRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): OpenChatServerRequest;

    /**
     * Verifies an OpenChatServerRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates an OpenChatServerRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns OpenChatServerRequest
     */
    public static fromObject(object: { [k: string]: any }): OpenChatServerRequest;

    /**
     * Creates a plain object from an OpenChatServerRequest message. Also converts values to other types if specified.
     * @param message OpenChatServerRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: OpenChatServerRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this OpenChatServerRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a ChatServerEvent. */
export interface IChatServerEvent {

    /** ChatServerEvent open */
    open?: (ChatServerEvent.IOpen|null);

    /** ChatServerEvent close */
    close?: (ChatServerEvent.IClose|null);
}

/** Represents a ChatServerEvent. */
export class ChatServerEvent implements IChatServerEvent {

    /**
     * Constructs a new ChatServerEvent.
     * @param [properties] Properties to set
     */
    constructor(properties?: IChatServerEvent);

    /** ChatServerEvent open. */
    public open?: (ChatServerEvent.IOpen|null);

    /** ChatServerEvent close. */
    public close?: (ChatServerEvent.IClose|null);

    /** ChatServerEvent body. */
    public body?: ("open"|"close");

    /**
     * Creates a new ChatServerEvent instance using the specified properties.
     * @param [properties] Properties to set
     * @returns ChatServerEvent instance
     */
    public static create(properties?: IChatServerEvent): ChatServerEvent;

    /**
     * Encodes the specified ChatServerEvent message. Does not implicitly {@link ChatServerEvent.verify|verify} messages.
     * @param message ChatServerEvent message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IChatServerEvent, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified ChatServerEvent message, length delimited. Does not implicitly {@link ChatServerEvent.verify|verify} messages.
     * @param message ChatServerEvent message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IChatServerEvent, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a ChatServerEvent message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns ChatServerEvent
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): ChatServerEvent;

    /**
     * Decodes a ChatServerEvent message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns ChatServerEvent
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): ChatServerEvent;

    /**
     * Verifies a ChatServerEvent message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a ChatServerEvent message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns ChatServerEvent
     */
    public static fromObject(object: { [k: string]: any }): ChatServerEvent;

    /**
     * Creates a plain object from a ChatServerEvent message. Also converts values to other types if specified.
     * @param message ChatServerEvent
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: ChatServerEvent, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this ChatServerEvent to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

export namespace ChatServerEvent {

    /** Properties of an Open. */
    interface IOpen {

        /** Open serverId */
        serverId?: (number|null);
    }

    /** Represents an Open. */
    class Open implements IOpen {

        /**
         * Constructs a new Open.
         * @param [properties] Properties to set
         */
        constructor(properties?: ChatServerEvent.IOpen);

        /** Open serverId. */
        public serverId: number;

        /**
         * Creates a new Open instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Open instance
         */
        public static create(properties?: ChatServerEvent.IOpen): ChatServerEvent.Open;

        /**
         * Encodes the specified Open message. Does not implicitly {@link ChatServerEvent.Open.verify|verify} messages.
         * @param message Open message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: ChatServerEvent.IOpen, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Open message, length delimited. Does not implicitly {@link ChatServerEvent.Open.verify|verify} messages.
         * @param message Open message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: ChatServerEvent.IOpen, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes an Open message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Open
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): ChatServerEvent.Open;

        /**
         * Decodes an Open message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Open
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): ChatServerEvent.Open;

        /**
         * Verifies an Open message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates an Open message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Open
         */
        public static fromObject(object: { [k: string]: any }): ChatServerEvent.Open;

        /**
         * Creates a plain object from an Open message. Also converts values to other types if specified.
         * @param message Open
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: ChatServerEvent.Open, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Open to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }

    /** Properties of a Close. */
    interface IClose {
    }

    /** Represents a Close. */
    class Close implements IClose {

        /**
         * Constructs a new Close.
         * @param [properties] Properties to set
         */
        constructor(properties?: ChatServerEvent.IClose);

        /**
         * Creates a new Close instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Close instance
         */
        public static create(properties?: ChatServerEvent.IClose): ChatServerEvent.Close;

        /**
         * Encodes the specified Close message. Does not implicitly {@link ChatServerEvent.Close.verify|verify} messages.
         * @param message Close message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: ChatServerEvent.IClose, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Close message, length delimited. Does not implicitly {@link ChatServerEvent.Close.verify|verify} messages.
         * @param message Close message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: ChatServerEvent.IClose, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a Close message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Close
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): ChatServerEvent.Close;

        /**
         * Decodes a Close message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Close
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): ChatServerEvent.Close;

        /**
         * Verifies a Close message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a Close message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Close
         */
        public static fromObject(object: { [k: string]: any }): ChatServerEvent.Close;

        /**
         * Creates a plain object from a Close message. Also converts values to other types if specified.
         * @param message Close
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: ChatServerEvent.Close, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Close to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }
}

/** Properties of a CallChatServerRequest. */
export interface ICallChatServerRequest {

    /** CallChatServerRequest serverId */
    serverId?: (number|null);

    /** CallChatServerRequest close */
    close?: (CallChatServerRequest.IClose|null);
}

/** Represents a CallChatServerRequest. */
export class CallChatServerRequest implements ICallChatServerRequest {

    /**
     * Constructs a new CallChatServerRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: ICallChatServerRequest);

    /** CallChatServerRequest serverId. */
    public serverId: number;

    /** CallChatServerRequest close. */
    public close?: (CallChatServerRequest.IClose|null);

    /** CallChatServerRequest body. */
    public body?: "close";

    /**
     * Creates a new CallChatServerRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns CallChatServerRequest instance
     */
    public static create(properties?: ICallChatServerRequest): CallChatServerRequest;

    /**
     * Encodes the specified CallChatServerRequest message. Does not implicitly {@link CallChatServerRequest.verify|verify} messages.
     * @param message CallChatServerRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: ICallChatServerRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified CallChatServerRequest message, length delimited. Does not implicitly {@link CallChatServerRequest.verify|verify} messages.
     * @param message CallChatServerRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: ICallChatServerRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a CallChatServerRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns CallChatServerRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): CallChatServerRequest;

    /**
     * Decodes a CallChatServerRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns CallChatServerRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): CallChatServerRequest;

    /**
     * Verifies a CallChatServerRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a CallChatServerRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns CallChatServerRequest
     */
    public static fromObject(object: { [k: string]: any }): CallChatServerRequest;

    /**
     * Creates a plain object from a CallChatServerRequest message. Also converts values to other types if specified.
     * @param message CallChatServerRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: CallChatServerRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this CallChatServerRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

export namespace CallChatServerRequest {

    /** Properties of a Close. */
    interface IClose {
    }

    /** Represents a Close. */
    class Close implements IClose {

        /**
         * Constructs a new Close.
         * @param [properties] Properties to set
         */
        constructor(properties?: CallChatServerRequest.IClose);

        /**
         * Creates a new Close instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Close instance
         */
        public static create(properties?: CallChatServerRequest.IClose): CallChatServerRequest.Close;

        /**
         * Encodes the specified Close message. Does not implicitly {@link CallChatServerRequest.Close.verify|verify} messages.
         * @param message Close message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: CallChatServerRequest.IClose, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Close message, length delimited. Does not implicitly {@link CallChatServerRequest.Close.verify|verify} messages.
         * @param message Close message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: CallChatServerRequest.IClose, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a Close message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Close
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): CallChatServerRequest.Close;

        /**
         * Decodes a Close message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Close
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): CallChatServerRequest.Close;

        /**
         * Verifies a Close message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a Close message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Close
         */
        public static fromObject(object: { [k: string]: any }): CallChatServerRequest.Close;

        /**
         * Creates a plain object from a Close message. Also converts values to other types if specified.
         * @param message Close
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: CallChatServerRequest.Close, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Close to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }
}

/** Properties of an OpenChatClientRequest. */
export interface IOpenChatClientRequest {

    /** OpenChatClientRequest networkKey */
    networkKey?: (Uint8Array|null);

    /** OpenChatClientRequest serverKey */
    serverKey?: (Uint8Array|null);
}

/** Represents an OpenChatClientRequest. */
export class OpenChatClientRequest implements IOpenChatClientRequest {

    /**
     * Constructs a new OpenChatClientRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: IOpenChatClientRequest);

    /** OpenChatClientRequest networkKey. */
    public networkKey: Uint8Array;

    /** OpenChatClientRequest serverKey. */
    public serverKey: Uint8Array;

    /**
     * Creates a new OpenChatClientRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns OpenChatClientRequest instance
     */
    public static create(properties?: IOpenChatClientRequest): OpenChatClientRequest;

    /**
     * Encodes the specified OpenChatClientRequest message. Does not implicitly {@link OpenChatClientRequest.verify|verify} messages.
     * @param message OpenChatClientRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IOpenChatClientRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified OpenChatClientRequest message, length delimited. Does not implicitly {@link OpenChatClientRequest.verify|verify} messages.
     * @param message OpenChatClientRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IOpenChatClientRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes an OpenChatClientRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns OpenChatClientRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): OpenChatClientRequest;

    /**
     * Decodes an OpenChatClientRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns OpenChatClientRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): OpenChatClientRequest;

    /**
     * Verifies an OpenChatClientRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates an OpenChatClientRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns OpenChatClientRequest
     */
    public static fromObject(object: { [k: string]: any }): OpenChatClientRequest;

    /**
     * Creates a plain object from an OpenChatClientRequest message. Also converts values to other types if specified.
     * @param message OpenChatClientRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: OpenChatClientRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this OpenChatClientRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a ChatClientEvent. */
export interface IChatClientEvent {

    /** ChatClientEvent open */
    open?: (ChatClientEvent.IOpen|null);

    /** ChatClientEvent message */
    message?: (ChatClientEvent.IMessage|null);

    /** ChatClientEvent close */
    close?: (ChatClientEvent.IClose|null);
}

/** Represents a ChatClientEvent. */
export class ChatClientEvent implements IChatClientEvent {

    /**
     * Constructs a new ChatClientEvent.
     * @param [properties] Properties to set
     */
    constructor(properties?: IChatClientEvent);

    /** ChatClientEvent open. */
    public open?: (ChatClientEvent.IOpen|null);

    /** ChatClientEvent message. */
    public message?: (ChatClientEvent.IMessage|null);

    /** ChatClientEvent close. */
    public close?: (ChatClientEvent.IClose|null);

    /** ChatClientEvent body. */
    public body?: ("open"|"message"|"close");

    /**
     * Creates a new ChatClientEvent instance using the specified properties.
     * @param [properties] Properties to set
     * @returns ChatClientEvent instance
     */
    public static create(properties?: IChatClientEvent): ChatClientEvent;

    /**
     * Encodes the specified ChatClientEvent message. Does not implicitly {@link ChatClientEvent.verify|verify} messages.
     * @param message ChatClientEvent message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IChatClientEvent, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified ChatClientEvent message, length delimited. Does not implicitly {@link ChatClientEvent.verify|verify} messages.
     * @param message ChatClientEvent message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IChatClientEvent, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a ChatClientEvent message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns ChatClientEvent
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): ChatClientEvent;

    /**
     * Decodes a ChatClientEvent message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns ChatClientEvent
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): ChatClientEvent;

    /**
     * Verifies a ChatClientEvent message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a ChatClientEvent message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns ChatClientEvent
     */
    public static fromObject(object: { [k: string]: any }): ChatClientEvent;

    /**
     * Creates a plain object from a ChatClientEvent message. Also converts values to other types if specified.
     * @param message ChatClientEvent
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: ChatClientEvent, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this ChatClientEvent to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

export namespace ChatClientEvent {

    /** Properties of an Open. */
    interface IOpen {

        /** Open clientId */
        clientId?: (number|null);
    }

    /** Represents an Open. */
    class Open implements IOpen {

        /**
         * Constructs a new Open.
         * @param [properties] Properties to set
         */
        constructor(properties?: ChatClientEvent.IOpen);

        /** Open clientId. */
        public clientId: number;

        /**
         * Creates a new Open instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Open instance
         */
        public static create(properties?: ChatClientEvent.IOpen): ChatClientEvent.Open;

        /**
         * Encodes the specified Open message. Does not implicitly {@link ChatClientEvent.Open.verify|verify} messages.
         * @param message Open message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: ChatClientEvent.IOpen, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Open message, length delimited. Does not implicitly {@link ChatClientEvent.Open.verify|verify} messages.
         * @param message Open message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: ChatClientEvent.IOpen, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes an Open message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Open
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): ChatClientEvent.Open;

        /**
         * Decodes an Open message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Open
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): ChatClientEvent.Open;

        /**
         * Verifies an Open message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates an Open message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Open
         */
        public static fromObject(object: { [k: string]: any }): ChatClientEvent.Open;

        /**
         * Creates a plain object from an Open message. Also converts values to other types if specified.
         * @param message Open
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: ChatClientEvent.Open, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Open to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }

    /** Properties of a Message. */
    interface IMessage {

        /** Message sentTime */
        sentTime?: (number|null);

        /** Message serverTime */
        serverTime?: (number|null);

        /** Message nick */
        nick?: (string|null);

        /** Message body */
        body?: (string|null);

        /** Message entities */
        entities?: (IMessageEntities|null);
    }

    /** Represents a Message. */
    class Message implements IMessage {

        /**
         * Constructs a new Message.
         * @param [properties] Properties to set
         */
        constructor(properties?: ChatClientEvent.IMessage);

        /** Message sentTime. */
        public sentTime: number;

        /** Message serverTime. */
        public serverTime: number;

        /** Message nick. */
        public nick: string;

        /** Message body. */
        public body: string;

        /** Message entities. */
        public entities?: (IMessageEntities|null);

        /**
         * Creates a new Message instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Message instance
         */
        public static create(properties?: ChatClientEvent.IMessage): ChatClientEvent.Message;

        /**
         * Encodes the specified Message message. Does not implicitly {@link ChatClientEvent.Message.verify|verify} messages.
         * @param message Message message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: ChatClientEvent.IMessage, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Message message, length delimited. Does not implicitly {@link ChatClientEvent.Message.verify|verify} messages.
         * @param message Message message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: ChatClientEvent.IMessage, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a Message message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Message
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): ChatClientEvent.Message;

        /**
         * Decodes a Message message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Message
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): ChatClientEvent.Message;

        /**
         * Verifies a Message message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a Message message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Message
         */
        public static fromObject(object: { [k: string]: any }): ChatClientEvent.Message;

        /**
         * Creates a plain object from a Message message. Also converts values to other types if specified.
         * @param message Message
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: ChatClientEvent.Message, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Message to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }

    /** Properties of a Close. */
    interface IClose {
    }

    /** Represents a Close. */
    class Close implements IClose {

        /**
         * Constructs a new Close.
         * @param [properties] Properties to set
         */
        constructor(properties?: ChatClientEvent.IClose);

        /**
         * Creates a new Close instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Close instance
         */
        public static create(properties?: ChatClientEvent.IClose): ChatClientEvent.Close;

        /**
         * Encodes the specified Close message. Does not implicitly {@link ChatClientEvent.Close.verify|verify} messages.
         * @param message Close message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: ChatClientEvent.IClose, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Close message, length delimited. Does not implicitly {@link ChatClientEvent.Close.verify|verify} messages.
         * @param message Close message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: ChatClientEvent.IClose, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a Close message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Close
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): ChatClientEvent.Close;

        /**
         * Decodes a Close message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Close
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): ChatClientEvent.Close;

        /**
         * Verifies a Close message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a Close message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Close
         */
        public static fromObject(object: { [k: string]: any }): ChatClientEvent.Close;

        /**
         * Creates a plain object from a Close message. Also converts values to other types if specified.
         * @param message Close
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: ChatClientEvent.Close, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Close to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }
}

/** Properties of a ChatRoom. */
export interface IChatRoom {

    /** ChatRoom name */
    name?: (string|null);
}

/** Represents a ChatRoom. */
export class ChatRoom implements IChatRoom {

    /**
     * Constructs a new ChatRoom.
     * @param [properties] Properties to set
     */
    constructor(properties?: IChatRoom);

    /** ChatRoom name. */
    public name: string;

    /**
     * Creates a new ChatRoom instance using the specified properties.
     * @param [properties] Properties to set
     * @returns ChatRoom instance
     */
    public static create(properties?: IChatRoom): ChatRoom;

    /**
     * Encodes the specified ChatRoom message. Does not implicitly {@link ChatRoom.verify|verify} messages.
     * @param message ChatRoom message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IChatRoom, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified ChatRoom message, length delimited. Does not implicitly {@link ChatRoom.verify|verify} messages.
     * @param message ChatRoom message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IChatRoom, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a ChatRoom message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns ChatRoom
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): ChatRoom;

    /**
     * Decodes a ChatRoom message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns ChatRoom
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): ChatRoom;

    /**
     * Verifies a ChatRoom message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a ChatRoom message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns ChatRoom
     */
    public static fromObject(object: { [k: string]: any }): ChatRoom;

    /**
     * Creates a plain object from a ChatRoom message. Also converts values to other types if specified.
     * @param message ChatRoom
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: ChatRoom, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this ChatRoom to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a ChatServer. */
export interface IChatServer {

    /** ChatServer id */
    id?: (number|null);

    /** ChatServer networkKey */
    networkKey?: (Uint8Array|null);

    /** ChatServer key */
    key?: (IKey|null);

    /** ChatServer chatRoom */
    chatRoom?: (IChatRoom|null);
}

/** Represents a ChatServer. */
export class ChatServer implements IChatServer {

    /**
     * Constructs a new ChatServer.
     * @param [properties] Properties to set
     */
    constructor(properties?: IChatServer);

    /** ChatServer id. */
    public id: number;

    /** ChatServer networkKey. */
    public networkKey: Uint8Array;

    /** ChatServer key. */
    public key?: (IKey|null);

    /** ChatServer chatRoom. */
    public chatRoom?: (IChatRoom|null);

    /**
     * Creates a new ChatServer instance using the specified properties.
     * @param [properties] Properties to set
     * @returns ChatServer instance
     */
    public static create(properties?: IChatServer): ChatServer;

    /**
     * Encodes the specified ChatServer message. Does not implicitly {@link ChatServer.verify|verify} messages.
     * @param message ChatServer message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IChatServer, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified ChatServer message, length delimited. Does not implicitly {@link ChatServer.verify|verify} messages.
     * @param message ChatServer message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IChatServer, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a ChatServer message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns ChatServer
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): ChatServer;

    /**
     * Decodes a ChatServer message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns ChatServer
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): ChatServer;

    /**
     * Verifies a ChatServer message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a ChatServer message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns ChatServer
     */
    public static fromObject(object: { [k: string]: any }): ChatServer;

    /**
     * Creates a plain object from a ChatServer message. Also converts values to other types if specified.
     * @param message ChatServer
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: ChatServer, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this ChatServer to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a MessageEntities. */
export interface IMessageEntities {

    /** MessageEntities links */
    links?: (MessageEntities.ILink[]|null);

    /** MessageEntities emotes */
    emotes?: (MessageEntities.IEmote[]|null);

    /** MessageEntities nicks */
    nicks?: (MessageEntities.INick[]|null);

    /** MessageEntities tags */
    tags?: (MessageEntities.ITag[]|null);

    /** MessageEntities codeBlocks */
    codeBlocks?: (MessageEntities.ICodeBlock[]|null);

    /** MessageEntities spoilers */
    spoilers?: (MessageEntities.ISpoiler[]|null);

    /** MessageEntities greenText */
    greenText?: (MessageEntities.IGenericEntity|null);

    /** MessageEntities selfMessage */
    selfMessage?: (MessageEntities.IGenericEntity|null);
}

/** Represents a MessageEntities. */
export class MessageEntities implements IMessageEntities {

    /**
     * Constructs a new MessageEntities.
     * @param [properties] Properties to set
     */
    constructor(properties?: IMessageEntities);

    /** MessageEntities links. */
    public links: MessageEntities.ILink[];

    /** MessageEntities emotes. */
    public emotes: MessageEntities.IEmote[];

    /** MessageEntities nicks. */
    public nicks: MessageEntities.INick[];

    /** MessageEntities tags. */
    public tags: MessageEntities.ITag[];

    /** MessageEntities codeBlocks. */
    public codeBlocks: MessageEntities.ICodeBlock[];

    /** MessageEntities spoilers. */
    public spoilers: MessageEntities.ISpoiler[];

    /** MessageEntities greenText. */
    public greenText?: (MessageEntities.IGenericEntity|null);

    /** MessageEntities selfMessage. */
    public selfMessage?: (MessageEntities.IGenericEntity|null);

    /**
     * Creates a new MessageEntities instance using the specified properties.
     * @param [properties] Properties to set
     * @returns MessageEntities instance
     */
    public static create(properties?: IMessageEntities): MessageEntities;

    /**
     * Encodes the specified MessageEntities message. Does not implicitly {@link MessageEntities.verify|verify} messages.
     * @param message MessageEntities message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IMessageEntities, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified MessageEntities message, length delimited. Does not implicitly {@link MessageEntities.verify|verify} messages.
     * @param message MessageEntities message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IMessageEntities, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a MessageEntities message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns MessageEntities
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): MessageEntities;

    /**
     * Decodes a MessageEntities message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns MessageEntities
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): MessageEntities;

    /**
     * Verifies a MessageEntities message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a MessageEntities message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns MessageEntities
     */
    public static fromObject(object: { [k: string]: any }): MessageEntities;

    /**
     * Creates a plain object from a MessageEntities message. Also converts values to other types if specified.
     * @param message MessageEntities
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: MessageEntities, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this MessageEntities to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

export namespace MessageEntities {

    /** Properties of a Bounds. */
    interface IBounds {

        /** Bounds start */
        start?: (number|null);

        /** Bounds end */
        end?: (number|null);
    }

    /** Represents a Bounds. */
    class Bounds implements IBounds {

        /**
         * Constructs a new Bounds.
         * @param [properties] Properties to set
         */
        constructor(properties?: MessageEntities.IBounds);

        /** Bounds start. */
        public start: number;

        /** Bounds end. */
        public end: number;

        /**
         * Creates a new Bounds instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Bounds instance
         */
        public static create(properties?: MessageEntities.IBounds): MessageEntities.Bounds;

        /**
         * Encodes the specified Bounds message. Does not implicitly {@link MessageEntities.Bounds.verify|verify} messages.
         * @param message Bounds message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: MessageEntities.IBounds, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Bounds message, length delimited. Does not implicitly {@link MessageEntities.Bounds.verify|verify} messages.
         * @param message Bounds message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: MessageEntities.IBounds, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a Bounds message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Bounds
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): MessageEntities.Bounds;

        /**
         * Decodes a Bounds message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Bounds
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): MessageEntities.Bounds;

        /**
         * Verifies a Bounds message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a Bounds message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Bounds
         */
        public static fromObject(object: { [k: string]: any }): MessageEntities.Bounds;

        /**
         * Creates a plain object from a Bounds message. Also converts values to other types if specified.
         * @param message Bounds
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: MessageEntities.Bounds, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Bounds to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }

    /** Properties of a Link. */
    interface ILink {

        /** Link bounds */
        bounds?: (MessageEntities.IBounds|null);

        /** Link url */
        url?: (string|null);
    }

    /** Represents a Link. */
    class Link implements ILink {

        /**
         * Constructs a new Link.
         * @param [properties] Properties to set
         */
        constructor(properties?: MessageEntities.ILink);

        /** Link bounds. */
        public bounds?: (MessageEntities.IBounds|null);

        /** Link url. */
        public url: string;

        /**
         * Creates a new Link instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Link instance
         */
        public static create(properties?: MessageEntities.ILink): MessageEntities.Link;

        /**
         * Encodes the specified Link message. Does not implicitly {@link MessageEntities.Link.verify|verify} messages.
         * @param message Link message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: MessageEntities.ILink, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Link message, length delimited. Does not implicitly {@link MessageEntities.Link.verify|verify} messages.
         * @param message Link message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: MessageEntities.ILink, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a Link message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Link
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): MessageEntities.Link;

        /**
         * Decodes a Link message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Link
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): MessageEntities.Link;

        /**
         * Verifies a Link message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a Link message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Link
         */
        public static fromObject(object: { [k: string]: any }): MessageEntities.Link;

        /**
         * Creates a plain object from a Link message. Also converts values to other types if specified.
         * @param message Link
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: MessageEntities.Link, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Link to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }

    /** Properties of an Emote. */
    interface IEmote {

        /** Emote bounds */
        bounds?: (MessageEntities.IBounds|null);

        /** Emote name */
        name?: (string|null);

        /** Emote modifiers */
        modifiers?: (string[]|null);

        /** Emote combo */
        combo?: (number|null);
    }

    /** Represents an Emote. */
    class Emote implements IEmote {

        /**
         * Constructs a new Emote.
         * @param [properties] Properties to set
         */
        constructor(properties?: MessageEntities.IEmote);

        /** Emote bounds. */
        public bounds?: (MessageEntities.IBounds|null);

        /** Emote name. */
        public name: string;

        /** Emote modifiers. */
        public modifiers: string[];

        /** Emote combo. */
        public combo: number;

        /**
         * Creates a new Emote instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Emote instance
         */
        public static create(properties?: MessageEntities.IEmote): MessageEntities.Emote;

        /**
         * Encodes the specified Emote message. Does not implicitly {@link MessageEntities.Emote.verify|verify} messages.
         * @param message Emote message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: MessageEntities.IEmote, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Emote message, length delimited. Does not implicitly {@link MessageEntities.Emote.verify|verify} messages.
         * @param message Emote message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: MessageEntities.IEmote, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes an Emote message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Emote
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): MessageEntities.Emote;

        /**
         * Decodes an Emote message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Emote
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): MessageEntities.Emote;

        /**
         * Verifies an Emote message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates an Emote message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Emote
         */
        public static fromObject(object: { [k: string]: any }): MessageEntities.Emote;

        /**
         * Creates a plain object from an Emote message. Also converts values to other types if specified.
         * @param message Emote
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: MessageEntities.Emote, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Emote to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }

    /** Properties of a Nick. */
    interface INick {

        /** Nick bounds */
        bounds?: (MessageEntities.IBounds|null);

        /** Nick nick */
        nick?: (string|null);
    }

    /** Represents a Nick. */
    class Nick implements INick {

        /**
         * Constructs a new Nick.
         * @param [properties] Properties to set
         */
        constructor(properties?: MessageEntities.INick);

        /** Nick bounds. */
        public bounds?: (MessageEntities.IBounds|null);

        /** Nick nick. */
        public nick: string;

        /**
         * Creates a new Nick instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Nick instance
         */
        public static create(properties?: MessageEntities.INick): MessageEntities.Nick;

        /**
         * Encodes the specified Nick message. Does not implicitly {@link MessageEntities.Nick.verify|verify} messages.
         * @param message Nick message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: MessageEntities.INick, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Nick message, length delimited. Does not implicitly {@link MessageEntities.Nick.verify|verify} messages.
         * @param message Nick message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: MessageEntities.INick, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a Nick message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Nick
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): MessageEntities.Nick;

        /**
         * Decodes a Nick message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Nick
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): MessageEntities.Nick;

        /**
         * Verifies a Nick message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a Nick message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Nick
         */
        public static fromObject(object: { [k: string]: any }): MessageEntities.Nick;

        /**
         * Creates a plain object from a Nick message. Also converts values to other types if specified.
         * @param message Nick
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: MessageEntities.Nick, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Nick to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }

    /** Properties of a Tag. */
    interface ITag {

        /** Tag bounds */
        bounds?: (MessageEntities.IBounds|null);

        /** Tag name */
        name?: (string|null);
    }

    /** Represents a Tag. */
    class Tag implements ITag {

        /**
         * Constructs a new Tag.
         * @param [properties] Properties to set
         */
        constructor(properties?: MessageEntities.ITag);

        /** Tag bounds. */
        public bounds?: (MessageEntities.IBounds|null);

        /** Tag name. */
        public name: string;

        /**
         * Creates a new Tag instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Tag instance
         */
        public static create(properties?: MessageEntities.ITag): MessageEntities.Tag;

        /**
         * Encodes the specified Tag message. Does not implicitly {@link MessageEntities.Tag.verify|verify} messages.
         * @param message Tag message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: MessageEntities.ITag, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Tag message, length delimited. Does not implicitly {@link MessageEntities.Tag.verify|verify} messages.
         * @param message Tag message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: MessageEntities.ITag, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a Tag message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Tag
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): MessageEntities.Tag;

        /**
         * Decodes a Tag message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Tag
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): MessageEntities.Tag;

        /**
         * Verifies a Tag message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a Tag message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Tag
         */
        public static fromObject(object: { [k: string]: any }): MessageEntities.Tag;

        /**
         * Creates a plain object from a Tag message. Also converts values to other types if specified.
         * @param message Tag
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: MessageEntities.Tag, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Tag to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }

    /** Properties of a CodeBlock. */
    interface ICodeBlock {

        /** CodeBlock bounds */
        bounds?: (MessageEntities.IBounds|null);
    }

    /** Represents a CodeBlock. */
    class CodeBlock implements ICodeBlock {

        /**
         * Constructs a new CodeBlock.
         * @param [properties] Properties to set
         */
        constructor(properties?: MessageEntities.ICodeBlock);

        /** CodeBlock bounds. */
        public bounds?: (MessageEntities.IBounds|null);

        /**
         * Creates a new CodeBlock instance using the specified properties.
         * @param [properties] Properties to set
         * @returns CodeBlock instance
         */
        public static create(properties?: MessageEntities.ICodeBlock): MessageEntities.CodeBlock;

        /**
         * Encodes the specified CodeBlock message. Does not implicitly {@link MessageEntities.CodeBlock.verify|verify} messages.
         * @param message CodeBlock message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: MessageEntities.ICodeBlock, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified CodeBlock message, length delimited. Does not implicitly {@link MessageEntities.CodeBlock.verify|verify} messages.
         * @param message CodeBlock message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: MessageEntities.ICodeBlock, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a CodeBlock message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns CodeBlock
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): MessageEntities.CodeBlock;

        /**
         * Decodes a CodeBlock message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns CodeBlock
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): MessageEntities.CodeBlock;

        /**
         * Verifies a CodeBlock message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a CodeBlock message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns CodeBlock
         */
        public static fromObject(object: { [k: string]: any }): MessageEntities.CodeBlock;

        /**
         * Creates a plain object from a CodeBlock message. Also converts values to other types if specified.
         * @param message CodeBlock
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: MessageEntities.CodeBlock, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this CodeBlock to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }

    /** Properties of a Spoiler. */
    interface ISpoiler {

        /** Spoiler bounds */
        bounds?: (MessageEntities.IBounds|null);
    }

    /** Represents a Spoiler. */
    class Spoiler implements ISpoiler {

        /**
         * Constructs a new Spoiler.
         * @param [properties] Properties to set
         */
        constructor(properties?: MessageEntities.ISpoiler);

        /** Spoiler bounds. */
        public bounds?: (MessageEntities.IBounds|null);

        /**
         * Creates a new Spoiler instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Spoiler instance
         */
        public static create(properties?: MessageEntities.ISpoiler): MessageEntities.Spoiler;

        /**
         * Encodes the specified Spoiler message. Does not implicitly {@link MessageEntities.Spoiler.verify|verify} messages.
         * @param message Spoiler message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: MessageEntities.ISpoiler, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Spoiler message, length delimited. Does not implicitly {@link MessageEntities.Spoiler.verify|verify} messages.
         * @param message Spoiler message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: MessageEntities.ISpoiler, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a Spoiler message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Spoiler
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): MessageEntities.Spoiler;

        /**
         * Decodes a Spoiler message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Spoiler
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): MessageEntities.Spoiler;

        /**
         * Verifies a Spoiler message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a Spoiler message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Spoiler
         */
        public static fromObject(object: { [k: string]: any }): MessageEntities.Spoiler;

        /**
         * Creates a plain object from a Spoiler message. Also converts values to other types if specified.
         * @param message Spoiler
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: MessageEntities.Spoiler, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Spoiler to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }

    /** Properties of a GenericEntity. */
    interface IGenericEntity {

        /** GenericEntity bounds */
        bounds?: (MessageEntities.IBounds|null);
    }

    /** Represents a GenericEntity. */
    class GenericEntity implements IGenericEntity {

        /**
         * Constructs a new GenericEntity.
         * @param [properties] Properties to set
         */
        constructor(properties?: MessageEntities.IGenericEntity);

        /** GenericEntity bounds. */
        public bounds?: (MessageEntities.IBounds|null);

        /**
         * Creates a new GenericEntity instance using the specified properties.
         * @param [properties] Properties to set
         * @returns GenericEntity instance
         */
        public static create(properties?: MessageEntities.IGenericEntity): MessageEntities.GenericEntity;

        /**
         * Encodes the specified GenericEntity message. Does not implicitly {@link MessageEntities.GenericEntity.verify|verify} messages.
         * @param message GenericEntity message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: MessageEntities.IGenericEntity, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified GenericEntity message, length delimited. Does not implicitly {@link MessageEntities.GenericEntity.verify|verify} messages.
         * @param message GenericEntity message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: MessageEntities.IGenericEntity, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a GenericEntity message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns GenericEntity
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): MessageEntities.GenericEntity;

        /**
         * Decodes a GenericEntity message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns GenericEntity
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): MessageEntities.GenericEntity;

        /**
         * Verifies a GenericEntity message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a GenericEntity message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns GenericEntity
         */
        public static fromObject(object: { [k: string]: any }): MessageEntities.GenericEntity;

        /**
         * Creates a plain object from a GenericEntity message. Also converts values to other types if specified.
         * @param message GenericEntity
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: MessageEntities.GenericEntity, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this GenericEntity to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }
}

/** Properties of a CallChatClientRequest. */
export interface ICallChatClientRequest {

    /** CallChatClientRequest clientId */
    clientId?: (number|null);

    /** CallChatClientRequest message */
    message?: (CallChatClientRequest.IMessage|null);

    /** CallChatClientRequest close */
    close?: (CallChatClientRequest.IClose|null);
}

/** Represents a CallChatClientRequest. */
export class CallChatClientRequest implements ICallChatClientRequest {

    /**
     * Constructs a new CallChatClientRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: ICallChatClientRequest);

    /** CallChatClientRequest clientId. */
    public clientId: number;

    /** CallChatClientRequest message. */
    public message?: (CallChatClientRequest.IMessage|null);

    /** CallChatClientRequest close. */
    public close?: (CallChatClientRequest.IClose|null);

    /** CallChatClientRequest body. */
    public body?: ("message"|"close");

    /**
     * Creates a new CallChatClientRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns CallChatClientRequest instance
     */
    public static create(properties?: ICallChatClientRequest): CallChatClientRequest;

    /**
     * Encodes the specified CallChatClientRequest message. Does not implicitly {@link CallChatClientRequest.verify|verify} messages.
     * @param message CallChatClientRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: ICallChatClientRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified CallChatClientRequest message, length delimited. Does not implicitly {@link CallChatClientRequest.verify|verify} messages.
     * @param message CallChatClientRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: ICallChatClientRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a CallChatClientRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns CallChatClientRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): CallChatClientRequest;

    /**
     * Decodes a CallChatClientRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns CallChatClientRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): CallChatClientRequest;

    /**
     * Verifies a CallChatClientRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a CallChatClientRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns CallChatClientRequest
     */
    public static fromObject(object: { [k: string]: any }): CallChatClientRequest;

    /**
     * Creates a plain object from a CallChatClientRequest message. Also converts values to other types if specified.
     * @param message CallChatClientRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: CallChatClientRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this CallChatClientRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

export namespace CallChatClientRequest {

    /** Properties of a Message. */
    interface IMessage {

        /** Message time */
        time?: (number|null);

        /** Message body */
        body?: (string|null);
    }

    /** Represents a Message. */
    class Message implements IMessage {

        /**
         * Constructs a new Message.
         * @param [properties] Properties to set
         */
        constructor(properties?: CallChatClientRequest.IMessage);

        /** Message time. */
        public time: number;

        /** Message body. */
        public body: string;

        /**
         * Creates a new Message instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Message instance
         */
        public static create(properties?: CallChatClientRequest.IMessage): CallChatClientRequest.Message;

        /**
         * Encodes the specified Message message. Does not implicitly {@link CallChatClientRequest.Message.verify|verify} messages.
         * @param message Message message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: CallChatClientRequest.IMessage, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Message message, length delimited. Does not implicitly {@link CallChatClientRequest.Message.verify|verify} messages.
         * @param message Message message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: CallChatClientRequest.IMessage, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a Message message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Message
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): CallChatClientRequest.Message;

        /**
         * Decodes a Message message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Message
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): CallChatClientRequest.Message;

        /**
         * Verifies a Message message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a Message message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Message
         */
        public static fromObject(object: { [k: string]: any }): CallChatClientRequest.Message;

        /**
         * Creates a plain object from a Message message. Also converts values to other types if specified.
         * @param message Message
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: CallChatClientRequest.Message, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Message to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }

    /** Properties of a Close. */
    interface IClose {
    }

    /** Represents a Close. */
    class Close implements IClose {

        /**
         * Constructs a new Close.
         * @param [properties] Properties to set
         */
        constructor(properties?: CallChatClientRequest.IClose);

        /**
         * Creates a new Close instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Close instance
         */
        public static create(properties?: CallChatClientRequest.IClose): CallChatClientRequest.Close;

        /**
         * Encodes the specified Close message. Does not implicitly {@link CallChatClientRequest.Close.verify|verify} messages.
         * @param message Close message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: CallChatClientRequest.IClose, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Close message, length delimited. Does not implicitly {@link CallChatClientRequest.Close.verify|verify} messages.
         * @param message Close message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: CallChatClientRequest.IClose, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a Close message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Close
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): CallChatClientRequest.Close;

        /**
         * Decodes a Close message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Close
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): CallChatClientRequest.Close;

        /**
         * Verifies a Close message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a Close message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Close
         */
        public static fromObject(object: { [k: string]: any }): CallChatClientRequest.Close;

        /**
         * Creates a plain object from a Close message. Also converts values to other types if specified.
         * @param message Close
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: CallChatClientRequest.Close, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Close to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }
}

/** Properties of a Call. */
export interface ICall {

    /** Call id */
    id?: (number|null);

    /** Call parentId */
    parentId?: (number|null);

    /** Call method */
    method?: (string|null);

    /** Call argument */
    argument?: (google.protobuf.IAny|null);
}

/** Represents a Call. */
export class Call implements ICall {

    /**
     * Constructs a new Call.
     * @param [properties] Properties to set
     */
    constructor(properties?: ICall);

    /** Call id. */
    public id: number;

    /** Call parentId. */
    public parentId: number;

    /** Call method. */
    public method: string;

    /** Call argument. */
    public argument?: (google.protobuf.IAny|null);

    /**
     * Creates a new Call instance using the specified properties.
     * @param [properties] Properties to set
     * @returns Call instance
     */
    public static create(properties?: ICall): Call;

    /**
     * Encodes the specified Call message. Does not implicitly {@link Call.verify|verify} messages.
     * @param message Call message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: ICall, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified Call message, length delimited. Does not implicitly {@link Call.verify|verify} messages.
     * @param message Call message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: ICall, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a Call message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns Call
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): Call;

    /**
     * Decodes a Call message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns Call
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): Call;

    /**
     * Verifies a Call message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a Call message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns Call
     */
    public static fromObject(object: { [k: string]: any }): Call;

    /**
     * Creates a plain object from a Call message. Also converts values to other types if specified.
     * @param message Call
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: Call, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this Call to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of an Error. */
export interface IError {

    /** Error message */
    message?: (string|null);
}

/** Represents an Error. */
export class Error implements IError {

    /**
     * Constructs a new Error.
     * @param [properties] Properties to set
     */
    constructor(properties?: IError);

    /** Error message. */
    public message: string;

    /**
     * Creates a new Error instance using the specified properties.
     * @param [properties] Properties to set
     * @returns Error instance
     */
    public static create(properties?: IError): Error;

    /**
     * Encodes the specified Error message. Does not implicitly {@link Error.verify|verify} messages.
     * @param message Error message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IError, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified Error message, length delimited. Does not implicitly {@link Error.verify|verify} messages.
     * @param message Error message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IError, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes an Error message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns Error
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): Error;

    /**
     * Decodes an Error message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns Error
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): Error;

    /**
     * Verifies an Error message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates an Error message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns Error
     */
    public static fromObject(object: { [k: string]: any }): Error;

    /**
     * Creates a plain object from an Error message. Also converts values to other types if specified.
     * @param message Error
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: Error, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this Error to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a Cancel. */
export interface ICancel {
}

/** Represents a Cancel. */
export class Cancel implements ICancel {

    /**
     * Constructs a new Cancel.
     * @param [properties] Properties to set
     */
    constructor(properties?: ICancel);

    /**
     * Creates a new Cancel instance using the specified properties.
     * @param [properties] Properties to set
     * @returns Cancel instance
     */
    public static create(properties?: ICancel): Cancel;

    /**
     * Encodes the specified Cancel message. Does not implicitly {@link Cancel.verify|verify} messages.
     * @param message Cancel message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: ICancel, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified Cancel message, length delimited. Does not implicitly {@link Cancel.verify|verify} messages.
     * @param message Cancel message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: ICancel, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a Cancel message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns Cancel
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): Cancel;

    /**
     * Decodes a Cancel message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns Cancel
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): Cancel;

    /**
     * Verifies a Cancel message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a Cancel message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns Cancel
     */
    public static fromObject(object: { [k: string]: any }): Cancel;

    /**
     * Creates a plain object from a Cancel message. Also converts values to other types if specified.
     * @param message Cancel
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: Cancel, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this Cancel to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of an Undefined. */
export interface IUndefined {
}

/** Represents an Undefined. */
export class Undefined implements IUndefined {

    /**
     * Constructs a new Undefined.
     * @param [properties] Properties to set
     */
    constructor(properties?: IUndefined);

    /**
     * Creates a new Undefined instance using the specified properties.
     * @param [properties] Properties to set
     * @returns Undefined instance
     */
    public static create(properties?: IUndefined): Undefined;

    /**
     * Encodes the specified Undefined message. Does not implicitly {@link Undefined.verify|verify} messages.
     * @param message Undefined message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IUndefined, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified Undefined message, length delimited. Does not implicitly {@link Undefined.verify|verify} messages.
     * @param message Undefined message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IUndefined, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes an Undefined message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns Undefined
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): Undefined;

    /**
     * Decodes an Undefined message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns Undefined
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): Undefined;

    /**
     * Verifies an Undefined message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates an Undefined message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns Undefined
     */
    public static fromObject(object: { [k: string]: any }): Undefined;

    /**
     * Creates a plain object from an Undefined message. Also converts values to other types if specified.
     * @param message Undefined
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: Undefined, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this Undefined to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a Close. */
export interface IClose {
}

/** Represents a Close. */
export class Close implements IClose {

    /**
     * Constructs a new Close.
     * @param [properties] Properties to set
     */
    constructor(properties?: IClose);

    /**
     * Creates a new Close instance using the specified properties.
     * @param [properties] Properties to set
     * @returns Close instance
     */
    public static create(properties?: IClose): Close;

    /**
     * Encodes the specified Close message. Does not implicitly {@link Close.verify|verify} messages.
     * @param message Close message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IClose, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified Close message, length delimited. Does not implicitly {@link Close.verify|verify} messages.
     * @param message Close message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IClose, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a Close message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns Close
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): Close;

    /**
     * Decodes a Close message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns Close
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): Close;

    /**
     * Verifies a Close message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a Close message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns Close
     */
    public static fromObject(object: { [k: string]: any }): Close;

    /**
     * Creates a plain object from a Close message. Also converts values to other types if specified.
     * @param message Close
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: Close, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this Close to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a StartVPNRequest. */
export interface IStartVPNRequest {

    /** StartVPNRequest enableBootstrapPublishing */
    enableBootstrapPublishing?: (boolean|null);
}

/** Represents a StartVPNRequest. */
export class StartVPNRequest implements IStartVPNRequest {

    /**
     * Constructs a new StartVPNRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: IStartVPNRequest);

    /** StartVPNRequest enableBootstrapPublishing. */
    public enableBootstrapPublishing: boolean;

    /**
     * Creates a new StartVPNRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns StartVPNRequest instance
     */
    public static create(properties?: IStartVPNRequest): StartVPNRequest;

    /**
     * Encodes the specified StartVPNRequest message. Does not implicitly {@link StartVPNRequest.verify|verify} messages.
     * @param message StartVPNRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IStartVPNRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified StartVPNRequest message, length delimited. Does not implicitly {@link StartVPNRequest.verify|verify} messages.
     * @param message StartVPNRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IStartVPNRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a StartVPNRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns StartVPNRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): StartVPNRequest;

    /**
     * Decodes a StartVPNRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns StartVPNRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): StartVPNRequest;

    /**
     * Verifies a StartVPNRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a StartVPNRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns StartVPNRequest
     */
    public static fromObject(object: { [k: string]: any }): StartVPNRequest;

    /**
     * Creates a plain object from a StartVPNRequest message. Also converts values to other types if specified.
     * @param message StartVPNRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: StartVPNRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this StartVPNRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a StartVPNResponse. */
export interface IStartVPNResponse {
}

/** Represents a StartVPNResponse. */
export class StartVPNResponse implements IStartVPNResponse {

    /**
     * Constructs a new StartVPNResponse.
     * @param [properties] Properties to set
     */
    constructor(properties?: IStartVPNResponse);

    /**
     * Creates a new StartVPNResponse instance using the specified properties.
     * @param [properties] Properties to set
     * @returns StartVPNResponse instance
     */
    public static create(properties?: IStartVPNResponse): StartVPNResponse;

    /**
     * Encodes the specified StartVPNResponse message. Does not implicitly {@link StartVPNResponse.verify|verify} messages.
     * @param message StartVPNResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IStartVPNResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified StartVPNResponse message, length delimited. Does not implicitly {@link StartVPNResponse.verify|verify} messages.
     * @param message StartVPNResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IStartVPNResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a StartVPNResponse message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns StartVPNResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): StartVPNResponse;

    /**
     * Decodes a StartVPNResponse message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns StartVPNResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): StartVPNResponse;

    /**
     * Verifies a StartVPNResponse message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a StartVPNResponse message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns StartVPNResponse
     */
    public static fromObject(object: { [k: string]: any }): StartVPNResponse;

    /**
     * Creates a plain object from a StartVPNResponse message. Also converts values to other types if specified.
     * @param message StartVPNResponse
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: StartVPNResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this StartVPNResponse to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a StopVPNRequest. */
export interface IStopVPNRequest {
}

/** Represents a StopVPNRequest. */
export class StopVPNRequest implements IStopVPNRequest {

    /**
     * Constructs a new StopVPNRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: IStopVPNRequest);

    /**
     * Creates a new StopVPNRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns StopVPNRequest instance
     */
    public static create(properties?: IStopVPNRequest): StopVPNRequest;

    /**
     * Encodes the specified StopVPNRequest message. Does not implicitly {@link StopVPNRequest.verify|verify} messages.
     * @param message StopVPNRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IStopVPNRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified StopVPNRequest message, length delimited. Does not implicitly {@link StopVPNRequest.verify|verify} messages.
     * @param message StopVPNRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IStopVPNRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a StopVPNRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns StopVPNRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): StopVPNRequest;

    /**
     * Decodes a StopVPNRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns StopVPNRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): StopVPNRequest;

    /**
     * Verifies a StopVPNRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a StopVPNRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns StopVPNRequest
     */
    public static fromObject(object: { [k: string]: any }): StopVPNRequest;

    /**
     * Creates a plain object from a StopVPNRequest message. Also converts values to other types if specified.
     * @param message StopVPNRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: StopVPNRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this StopVPNRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a StopVPNResponse. */
export interface IStopVPNResponse {
}

/** Represents a StopVPNResponse. */
export class StopVPNResponse implements IStopVPNResponse {

    /**
     * Constructs a new StopVPNResponse.
     * @param [properties] Properties to set
     */
    constructor(properties?: IStopVPNResponse);

    /**
     * Creates a new StopVPNResponse instance using the specified properties.
     * @param [properties] Properties to set
     * @returns StopVPNResponse instance
     */
    public static create(properties?: IStopVPNResponse): StopVPNResponse;

    /**
     * Encodes the specified StopVPNResponse message. Does not implicitly {@link StopVPNResponse.verify|verify} messages.
     * @param message StopVPNResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IStopVPNResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified StopVPNResponse message, length delimited. Does not implicitly {@link StopVPNResponse.verify|verify} messages.
     * @param message StopVPNResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IStopVPNResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a StopVPNResponse message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns StopVPNResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): StopVPNResponse;

    /**
     * Decodes a StopVPNResponse message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns StopVPNResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): StopVPNResponse;

    /**
     * Verifies a StopVPNResponse message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a StopVPNResponse message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns StopVPNResponse
     */
    public static fromObject(object: { [k: string]: any }): StopVPNResponse;

    /**
     * Creates a plain object from a StopVPNResponse message. Also converts values to other types if specified.
     * @param message StopVPNResponse
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: StopVPNResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this StopVPNResponse to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a NetworkEvent. */
export interface INetworkEvent {

    /** NetworkEvent networkOpen */
    networkOpen?: (NetworkEvent.INetworkOpen|null);

    /** NetworkEvent networkClose */
    networkClose?: (NetworkEvent.INetworkClose|null);
}

/** Represents a NetworkEvent. */
export class NetworkEvent implements INetworkEvent {

    /**
     * Constructs a new NetworkEvent.
     * @param [properties] Properties to set
     */
    constructor(properties?: INetworkEvent);

    /** NetworkEvent networkOpen. */
    public networkOpen?: (NetworkEvent.INetworkOpen|null);

    /** NetworkEvent networkClose. */
    public networkClose?: (NetworkEvent.INetworkClose|null);

    /** NetworkEvent body. */
    public body?: ("networkOpen"|"networkClose");

    /**
     * Creates a new NetworkEvent instance using the specified properties.
     * @param [properties] Properties to set
     * @returns NetworkEvent instance
     */
    public static create(properties?: INetworkEvent): NetworkEvent;

    /**
     * Encodes the specified NetworkEvent message. Does not implicitly {@link NetworkEvent.verify|verify} messages.
     * @param message NetworkEvent message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: INetworkEvent, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified NetworkEvent message, length delimited. Does not implicitly {@link NetworkEvent.verify|verify} messages.
     * @param message NetworkEvent message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: INetworkEvent, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a NetworkEvent message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns NetworkEvent
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): NetworkEvent;

    /**
     * Decodes a NetworkEvent message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns NetworkEvent
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): NetworkEvent;

    /**
     * Verifies a NetworkEvent message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a NetworkEvent message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns NetworkEvent
     */
    public static fromObject(object: { [k: string]: any }): NetworkEvent;

    /**
     * Creates a plain object from a NetworkEvent message. Also converts values to other types if specified.
     * @param message NetworkEvent
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: NetworkEvent, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this NetworkEvent to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

export namespace NetworkEvent {

    /** Properties of a NetworkOpen. */
    interface INetworkOpen {

        /** NetworkOpen networkId */
        networkId?: (number|null);

        /** NetworkOpen networkKey */
        networkKey?: (Uint8Array|null);
    }

    /** Represents a NetworkOpen. */
    class NetworkOpen implements INetworkOpen {

        /**
         * Constructs a new NetworkOpen.
         * @param [properties] Properties to set
         */
        constructor(properties?: NetworkEvent.INetworkOpen);

        /** NetworkOpen networkId. */
        public networkId: number;

        /** NetworkOpen networkKey. */
        public networkKey: Uint8Array;

        /**
         * Creates a new NetworkOpen instance using the specified properties.
         * @param [properties] Properties to set
         * @returns NetworkOpen instance
         */
        public static create(properties?: NetworkEvent.INetworkOpen): NetworkEvent.NetworkOpen;

        /**
         * Encodes the specified NetworkOpen message. Does not implicitly {@link NetworkEvent.NetworkOpen.verify|verify} messages.
         * @param message NetworkOpen message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: NetworkEvent.INetworkOpen, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified NetworkOpen message, length delimited. Does not implicitly {@link NetworkEvent.NetworkOpen.verify|verify} messages.
         * @param message NetworkOpen message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: NetworkEvent.INetworkOpen, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a NetworkOpen message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns NetworkOpen
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): NetworkEvent.NetworkOpen;

        /**
         * Decodes a NetworkOpen message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns NetworkOpen
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): NetworkEvent.NetworkOpen;

        /**
         * Verifies a NetworkOpen message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a NetworkOpen message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns NetworkOpen
         */
        public static fromObject(object: { [k: string]: any }): NetworkEvent.NetworkOpen;

        /**
         * Creates a plain object from a NetworkOpen message. Also converts values to other types if specified.
         * @param message NetworkOpen
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: NetworkEvent.NetworkOpen, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this NetworkOpen to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }

    /** Properties of a NetworkClose. */
    interface INetworkClose {

        /** NetworkClose networkId */
        networkId?: (number|null);
    }

    /** Represents a NetworkClose. */
    class NetworkClose implements INetworkClose {

        /**
         * Constructs a new NetworkClose.
         * @param [properties] Properties to set
         */
        constructor(properties?: NetworkEvent.INetworkClose);

        /** NetworkClose networkId. */
        public networkId: number;

        /**
         * Creates a new NetworkClose instance using the specified properties.
         * @param [properties] Properties to set
         * @returns NetworkClose instance
         */
        public static create(properties?: NetworkEvent.INetworkClose): NetworkEvent.NetworkClose;

        /**
         * Encodes the specified NetworkClose message. Does not implicitly {@link NetworkEvent.NetworkClose.verify|verify} messages.
         * @param message NetworkClose message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: NetworkEvent.INetworkClose, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified NetworkClose message, length delimited. Does not implicitly {@link NetworkEvent.NetworkClose.verify|verify} messages.
         * @param message NetworkClose message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: NetworkEvent.INetworkClose, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a NetworkClose message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns NetworkClose
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): NetworkEvent.NetworkClose;

        /**
         * Decodes a NetworkClose message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns NetworkClose
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): NetworkEvent.NetworkClose;

        /**
         * Verifies a NetworkClose message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a NetworkClose message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns NetworkClose
         */
        public static fromObject(object: { [k: string]: any }): NetworkEvent.NetworkClose;

        /**
         * Creates a plain object from a NetworkClose message. Also converts values to other types if specified.
         * @param message NetworkClose
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: NetworkEvent.NetworkClose, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this NetworkClose to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }
}

/** Properties of a NetworkAddress. */
export interface INetworkAddress {

    /** NetworkAddress hostId */
    hostId?: (Uint8Array|null);

    /** NetworkAddress port */
    port?: (number|null);
}

/** Represents a NetworkAddress. */
export class NetworkAddress implements INetworkAddress {

    /**
     * Constructs a new NetworkAddress.
     * @param [properties] Properties to set
     */
    constructor(properties?: INetworkAddress);

    /** NetworkAddress hostId. */
    public hostId: Uint8Array;

    /** NetworkAddress port. */
    public port: number;

    /**
     * Creates a new NetworkAddress instance using the specified properties.
     * @param [properties] Properties to set
     * @returns NetworkAddress instance
     */
    public static create(properties?: INetworkAddress): NetworkAddress;

    /**
     * Encodes the specified NetworkAddress message. Does not implicitly {@link NetworkAddress.verify|verify} messages.
     * @param message NetworkAddress message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: INetworkAddress, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified NetworkAddress message, length delimited. Does not implicitly {@link NetworkAddress.verify|verify} messages.
     * @param message NetworkAddress message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: INetworkAddress, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a NetworkAddress message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns NetworkAddress
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): NetworkAddress;

    /**
     * Decodes a NetworkAddress message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns NetworkAddress
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): NetworkAddress;

    /**
     * Verifies a NetworkAddress message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a NetworkAddress message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns NetworkAddress
     */
    public static fromObject(object: { [k: string]: any }): NetworkAddress;

    /**
     * Creates a plain object from a NetworkAddress message. Also converts values to other types if specified.
     * @param message NetworkAddress
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: NetworkAddress, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this NetworkAddress to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a PeerInit. */
export interface IPeerInit {

    /** PeerInit protocolVersion */
    protocolVersion?: (number|null);

    /** PeerInit certificate */
    certificate?: (ICertificate|null);

    /** PeerInit nodePlatform */
    nodePlatform?: (string|null);

    /** PeerInit nodeVersion */
    nodeVersion?: (string|null);
}

/** Represents a PeerInit. */
export class PeerInit implements IPeerInit {

    /**
     * Constructs a new PeerInit.
     * @param [properties] Properties to set
     */
    constructor(properties?: IPeerInit);

    /** PeerInit protocolVersion. */
    public protocolVersion: number;

    /** PeerInit certificate. */
    public certificate?: (ICertificate|null);

    /** PeerInit nodePlatform. */
    public nodePlatform: string;

    /** PeerInit nodeVersion. */
    public nodeVersion: string;

    /**
     * Creates a new PeerInit instance using the specified properties.
     * @param [properties] Properties to set
     * @returns PeerInit instance
     */
    public static create(properties?: IPeerInit): PeerInit;

    /**
     * Encodes the specified PeerInit message. Does not implicitly {@link PeerInit.verify|verify} messages.
     * @param message PeerInit message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IPeerInit, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified PeerInit message, length delimited. Does not implicitly {@link PeerInit.verify|verify} messages.
     * @param message PeerInit message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IPeerInit, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a PeerInit message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns PeerInit
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): PeerInit;

    /**
     * Decodes a PeerInit message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns PeerInit
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): PeerInit;

    /**
     * Verifies a PeerInit message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a PeerInit message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns PeerInit
     */
    public static fromObject(object: { [k: string]: any }): PeerInit;

    /**
     * Creates a plain object from a PeerInit message. Also converts values to other types if specified.
     * @param message PeerInit
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: PeerInit, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this PeerInit to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a NetworkHandshake. */
export interface INetworkHandshake {

    /** NetworkHandshake init */
    init?: (NetworkHandshake.IInit|null);

    /** NetworkHandshake networkBindings */
    networkBindings?: (NetworkHandshake.INetworkBindings|null);
}

/** Represents a NetworkHandshake. */
export class NetworkHandshake implements INetworkHandshake {

    /**
     * Constructs a new NetworkHandshake.
     * @param [properties] Properties to set
     */
    constructor(properties?: INetworkHandshake);

    /** NetworkHandshake init. */
    public init?: (NetworkHandshake.IInit|null);

    /** NetworkHandshake networkBindings. */
    public networkBindings?: (NetworkHandshake.INetworkBindings|null);

    /** NetworkHandshake body. */
    public body?: ("init"|"networkBindings");

    /**
     * Creates a new NetworkHandshake instance using the specified properties.
     * @param [properties] Properties to set
     * @returns NetworkHandshake instance
     */
    public static create(properties?: INetworkHandshake): NetworkHandshake;

    /**
     * Encodes the specified NetworkHandshake message. Does not implicitly {@link NetworkHandshake.verify|verify} messages.
     * @param message NetworkHandshake message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: INetworkHandshake, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified NetworkHandshake message, length delimited. Does not implicitly {@link NetworkHandshake.verify|verify} messages.
     * @param message NetworkHandshake message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: INetworkHandshake, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a NetworkHandshake message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns NetworkHandshake
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): NetworkHandshake;

    /**
     * Decodes a NetworkHandshake message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns NetworkHandshake
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): NetworkHandshake;

    /**
     * Verifies a NetworkHandshake message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a NetworkHandshake message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns NetworkHandshake
     */
    public static fromObject(object: { [k: string]: any }): NetworkHandshake;

    /**
     * Creates a plain object from a NetworkHandshake message. Also converts values to other types if specified.
     * @param message NetworkHandshake
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: NetworkHandshake, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this NetworkHandshake to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

export namespace NetworkHandshake {

    /** Properties of an Init. */
    interface IInit {

        /** Init keyCount */
        keyCount?: (number|null);
    }

    /** Represents an Init. */
    class Init implements IInit {

        /**
         * Constructs a new Init.
         * @param [properties] Properties to set
         */
        constructor(properties?: NetworkHandshake.IInit);

        /** Init keyCount. */
        public keyCount: number;

        /**
         * Creates a new Init instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Init instance
         */
        public static create(properties?: NetworkHandshake.IInit): NetworkHandshake.Init;

        /**
         * Encodes the specified Init message. Does not implicitly {@link NetworkHandshake.Init.verify|verify} messages.
         * @param message Init message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: NetworkHandshake.IInit, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Init message, length delimited. Does not implicitly {@link NetworkHandshake.Init.verify|verify} messages.
         * @param message Init message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: NetworkHandshake.IInit, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes an Init message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Init
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): NetworkHandshake.Init;

        /**
         * Decodes an Init message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Init
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): NetworkHandshake.Init;

        /**
         * Verifies an Init message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates an Init message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Init
         */
        public static fromObject(object: { [k: string]: any }): NetworkHandshake.Init;

        /**
         * Creates a plain object from an Init message. Also converts values to other types if specified.
         * @param message Init
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: NetworkHandshake.Init, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Init to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }

    /** Properties of a NetworkBinding. */
    interface INetworkBinding {

        /** NetworkBinding port */
        port?: (number|null);

        /** NetworkBinding certificate */
        certificate?: (ICertificate|null);
    }

    /** Represents a NetworkBinding. */
    class NetworkBinding implements INetworkBinding {

        /**
         * Constructs a new NetworkBinding.
         * @param [properties] Properties to set
         */
        constructor(properties?: NetworkHandshake.INetworkBinding);

        /** NetworkBinding port. */
        public port: number;

        /** NetworkBinding certificate. */
        public certificate?: (ICertificate|null);

        /**
         * Creates a new NetworkBinding instance using the specified properties.
         * @param [properties] Properties to set
         * @returns NetworkBinding instance
         */
        public static create(properties?: NetworkHandshake.INetworkBinding): NetworkHandshake.NetworkBinding;

        /**
         * Encodes the specified NetworkBinding message. Does not implicitly {@link NetworkHandshake.NetworkBinding.verify|verify} messages.
         * @param message NetworkBinding message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: NetworkHandshake.INetworkBinding, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified NetworkBinding message, length delimited. Does not implicitly {@link NetworkHandshake.NetworkBinding.verify|verify} messages.
         * @param message NetworkBinding message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: NetworkHandshake.INetworkBinding, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a NetworkBinding message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns NetworkBinding
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): NetworkHandshake.NetworkBinding;

        /**
         * Decodes a NetworkBinding message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns NetworkBinding
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): NetworkHandshake.NetworkBinding;

        /**
         * Verifies a NetworkBinding message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a NetworkBinding message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns NetworkBinding
         */
        public static fromObject(object: { [k: string]: any }): NetworkHandshake.NetworkBinding;

        /**
         * Creates a plain object from a NetworkBinding message. Also converts values to other types if specified.
         * @param message NetworkBinding
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: NetworkHandshake.NetworkBinding, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this NetworkBinding to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }

    /** Properties of a NetworkBindings. */
    interface INetworkBindings {

        /** NetworkBindings networkBindings */
        networkBindings?: (NetworkHandshake.INetworkBinding[]|null);
    }

    /** Represents a NetworkBindings. */
    class NetworkBindings implements INetworkBindings {

        /**
         * Constructs a new NetworkBindings.
         * @param [properties] Properties to set
         */
        constructor(properties?: NetworkHandshake.INetworkBindings);

        /** NetworkBindings networkBindings. */
        public networkBindings: NetworkHandshake.INetworkBinding[];

        /**
         * Creates a new NetworkBindings instance using the specified properties.
         * @param [properties] Properties to set
         * @returns NetworkBindings instance
         */
        public static create(properties?: NetworkHandshake.INetworkBindings): NetworkHandshake.NetworkBindings;

        /**
         * Encodes the specified NetworkBindings message. Does not implicitly {@link NetworkHandshake.NetworkBindings.verify|verify} messages.
         * @param message NetworkBindings message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: NetworkHandshake.INetworkBindings, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified NetworkBindings message, length delimited. Does not implicitly {@link NetworkHandshake.NetworkBindings.verify|verify} messages.
         * @param message NetworkBindings message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: NetworkHandshake.INetworkBindings, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a NetworkBindings message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns NetworkBindings
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): NetworkHandshake.NetworkBindings;

        /**
         * Decodes a NetworkBindings message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns NetworkBindings
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): NetworkHandshake.NetworkBindings;

        /**
         * Verifies a NetworkBindings message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a NetworkBindings message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns NetworkBindings
         */
        public static fromObject(object: { [k: string]: any }): NetworkHandshake.NetworkBindings;

        /**
         * Creates a plain object from a NetworkBindings message. Also converts values to other types if specified.
         * @param message NetworkBindings
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: NetworkHandshake.NetworkBindings, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this NetworkBindings to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }
}

/** Properties of a BrokerPeerRequest. */
export interface IBrokerPeerRequest {

    /** BrokerPeerRequest connMtu */
    connMtu?: (number|null);
}

/** Represents a BrokerPeerRequest. */
export class BrokerPeerRequest implements IBrokerPeerRequest {

    /**
     * Constructs a new BrokerPeerRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: IBrokerPeerRequest);

    /** BrokerPeerRequest connMtu. */
    public connMtu: number;

    /**
     * Creates a new BrokerPeerRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns BrokerPeerRequest instance
     */
    public static create(properties?: IBrokerPeerRequest): BrokerPeerRequest;

    /**
     * Encodes the specified BrokerPeerRequest message. Does not implicitly {@link BrokerPeerRequest.verify|verify} messages.
     * @param message BrokerPeerRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IBrokerPeerRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified BrokerPeerRequest message, length delimited. Does not implicitly {@link BrokerPeerRequest.verify|verify} messages.
     * @param message BrokerPeerRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IBrokerPeerRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a BrokerPeerRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns BrokerPeerRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): BrokerPeerRequest;

    /**
     * Decodes a BrokerPeerRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns BrokerPeerRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): BrokerPeerRequest;

    /**
     * Verifies a BrokerPeerRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a BrokerPeerRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns BrokerPeerRequest
     */
    public static fromObject(object: { [k: string]: any }): BrokerPeerRequest;

    /**
     * Creates a plain object from a BrokerPeerRequest message. Also converts values to other types if specified.
     * @param message BrokerPeerRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: BrokerPeerRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this BrokerPeerRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a BrokerPeerEvent. */
export interface IBrokerPeerEvent {

    /** BrokerPeerEvent open */
    open?: (BrokerPeerEvent.IOpen|null);

    /** BrokerPeerEvent data */
    data?: (BrokerPeerEvent.IData|null);

    /** BrokerPeerEvent initRequired */
    initRequired?: (BrokerPeerEvent.IInitRequired|null);

    /** BrokerPeerEvent keys */
    keys?: (BrokerPeerEvent.IKeys|null);
}

/** Represents a BrokerPeerEvent. */
export class BrokerPeerEvent implements IBrokerPeerEvent {

    /**
     * Constructs a new BrokerPeerEvent.
     * @param [properties] Properties to set
     */
    constructor(properties?: IBrokerPeerEvent);

    /** BrokerPeerEvent open. */
    public open?: (BrokerPeerEvent.IOpen|null);

    /** BrokerPeerEvent data. */
    public data?: (BrokerPeerEvent.IData|null);

    /** BrokerPeerEvent initRequired. */
    public initRequired?: (BrokerPeerEvent.IInitRequired|null);

    /** BrokerPeerEvent keys. */
    public keys?: (BrokerPeerEvent.IKeys|null);

    /** BrokerPeerEvent body. */
    public body?: ("open"|"data"|"initRequired"|"keys");

    /**
     * Creates a new BrokerPeerEvent instance using the specified properties.
     * @param [properties] Properties to set
     * @returns BrokerPeerEvent instance
     */
    public static create(properties?: IBrokerPeerEvent): BrokerPeerEvent;

    /**
     * Encodes the specified BrokerPeerEvent message. Does not implicitly {@link BrokerPeerEvent.verify|verify} messages.
     * @param message BrokerPeerEvent message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IBrokerPeerEvent, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified BrokerPeerEvent message, length delimited. Does not implicitly {@link BrokerPeerEvent.verify|verify} messages.
     * @param message BrokerPeerEvent message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IBrokerPeerEvent, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a BrokerPeerEvent message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns BrokerPeerEvent
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): BrokerPeerEvent;

    /**
     * Decodes a BrokerPeerEvent message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns BrokerPeerEvent
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): BrokerPeerEvent;

    /**
     * Verifies a BrokerPeerEvent message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a BrokerPeerEvent message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns BrokerPeerEvent
     */
    public static fromObject(object: { [k: string]: any }): BrokerPeerEvent;

    /**
     * Creates a plain object from a BrokerPeerEvent message. Also converts values to other types if specified.
     * @param message BrokerPeerEvent
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: BrokerPeerEvent, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this BrokerPeerEvent to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

export namespace BrokerPeerEvent {

    /** Properties of an Open. */
    interface IOpen {

        /** Open peerId */
        peerId?: (number|null);
    }

    /** Represents an Open. */
    class Open implements IOpen {

        /**
         * Constructs a new Open.
         * @param [properties] Properties to set
         */
        constructor(properties?: BrokerPeerEvent.IOpen);

        /** Open peerId. */
        public peerId: number;

        /**
         * Creates a new Open instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Open instance
         */
        public static create(properties?: BrokerPeerEvent.IOpen): BrokerPeerEvent.Open;

        /**
         * Encodes the specified Open message. Does not implicitly {@link BrokerPeerEvent.Open.verify|verify} messages.
         * @param message Open message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: BrokerPeerEvent.IOpen, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Open message, length delimited. Does not implicitly {@link BrokerPeerEvent.Open.verify|verify} messages.
         * @param message Open message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: BrokerPeerEvent.IOpen, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes an Open message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Open
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): BrokerPeerEvent.Open;

        /**
         * Decodes an Open message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Open
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): BrokerPeerEvent.Open;

        /**
         * Verifies an Open message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates an Open message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Open
         */
        public static fromObject(object: { [k: string]: any }): BrokerPeerEvent.Open;

        /**
         * Creates a plain object from an Open message. Also converts values to other types if specified.
         * @param message Open
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: BrokerPeerEvent.Open, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Open to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }

    /** Properties of a Data. */
    interface IData {

        /** Data data */
        data?: (Uint8Array|null);
    }

    /** Represents a Data. */
    class Data implements IData {

        /**
         * Constructs a new Data.
         * @param [properties] Properties to set
         */
        constructor(properties?: BrokerPeerEvent.IData);

        /** Data data. */
        public data: Uint8Array;

        /**
         * Creates a new Data instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Data instance
         */
        public static create(properties?: BrokerPeerEvent.IData): BrokerPeerEvent.Data;

        /**
         * Encodes the specified Data message. Does not implicitly {@link BrokerPeerEvent.Data.verify|verify} messages.
         * @param message Data message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: BrokerPeerEvent.IData, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Data message, length delimited. Does not implicitly {@link BrokerPeerEvent.Data.verify|verify} messages.
         * @param message Data message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: BrokerPeerEvent.IData, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a Data message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Data
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): BrokerPeerEvent.Data;

        /**
         * Decodes a Data message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Data
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): BrokerPeerEvent.Data;

        /**
         * Verifies a Data message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a Data message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Data
         */
        public static fromObject(object: { [k: string]: any }): BrokerPeerEvent.Data;

        /**
         * Creates a plain object from a Data message. Also converts values to other types if specified.
         * @param message Data
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: BrokerPeerEvent.Data, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Data to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }

    /** Properties of an InitRequired. */
    interface IInitRequired {

        /** InitRequired data */
        data?: (Uint8Array|null);
    }

    /** Represents an InitRequired. */
    class InitRequired implements IInitRequired {

        /**
         * Constructs a new InitRequired.
         * @param [properties] Properties to set
         */
        constructor(properties?: BrokerPeerEvent.IInitRequired);

        /** InitRequired data. */
        public data: Uint8Array;

        /**
         * Creates a new InitRequired instance using the specified properties.
         * @param [properties] Properties to set
         * @returns InitRequired instance
         */
        public static create(properties?: BrokerPeerEvent.IInitRequired): BrokerPeerEvent.InitRequired;

        /**
         * Encodes the specified InitRequired message. Does not implicitly {@link BrokerPeerEvent.InitRequired.verify|verify} messages.
         * @param message InitRequired message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: BrokerPeerEvent.IInitRequired, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified InitRequired message, length delimited. Does not implicitly {@link BrokerPeerEvent.InitRequired.verify|verify} messages.
         * @param message InitRequired message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: BrokerPeerEvent.IInitRequired, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes an InitRequired message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns InitRequired
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): BrokerPeerEvent.InitRequired;

        /**
         * Decodes an InitRequired message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns InitRequired
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): BrokerPeerEvent.InitRequired;

        /**
         * Verifies an InitRequired message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates an InitRequired message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns InitRequired
         */
        public static fromObject(object: { [k: string]: any }): BrokerPeerEvent.InitRequired;

        /**
         * Creates a plain object from an InitRequired message. Also converts values to other types if specified.
         * @param message InitRequired
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: BrokerPeerEvent.InitRequired, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this InitRequired to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }

    /** Properties of a Keys. */
    interface IKeys {

        /** Keys keys */
        keys?: (Uint8Array[]|null);
    }

    /** Represents a Keys. */
    class Keys implements IKeys {

        /**
         * Constructs a new Keys.
         * @param [properties] Properties to set
         */
        constructor(properties?: BrokerPeerEvent.IKeys);

        /** Keys keys. */
        public keys: Uint8Array[];

        /**
         * Creates a new Keys instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Keys instance
         */
        public static create(properties?: BrokerPeerEvent.IKeys): BrokerPeerEvent.Keys;

        /**
         * Encodes the specified Keys message. Does not implicitly {@link BrokerPeerEvent.Keys.verify|verify} messages.
         * @param message Keys message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: BrokerPeerEvent.IKeys, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Keys message, length delimited. Does not implicitly {@link BrokerPeerEvent.Keys.verify|verify} messages.
         * @param message Keys message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: BrokerPeerEvent.IKeys, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a Keys message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Keys
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): BrokerPeerEvent.Keys;

        /**
         * Decodes a Keys message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Keys
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): BrokerPeerEvent.Keys;

        /**
         * Verifies a Keys message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a Keys message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Keys
         */
        public static fromObject(object: { [k: string]: any }): BrokerPeerEvent.Keys;

        /**
         * Creates a plain object from a Keys message. Also converts values to other types if specified.
         * @param message Keys
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: BrokerPeerEvent.Keys, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Keys to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }
}

/** Properties of a BrokerPeerInitRequest. */
export interface IBrokerPeerInitRequest {

    /** BrokerPeerInitRequest peerId */
    peerId?: (number|null);

    /** BrokerPeerInitRequest preferSender */
    preferSender?: (boolean|null);

    /** BrokerPeerInitRequest keys */
    keys?: (Uint8Array[]|null);
}

/** Represents a BrokerPeerInitRequest. */
export class BrokerPeerInitRequest implements IBrokerPeerInitRequest {

    /**
     * Constructs a new BrokerPeerInitRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: IBrokerPeerInitRequest);

    /** BrokerPeerInitRequest peerId. */
    public peerId: number;

    /** BrokerPeerInitRequest preferSender. */
    public preferSender: boolean;

    /** BrokerPeerInitRequest keys. */
    public keys: Uint8Array[];

    /**
     * Creates a new BrokerPeerInitRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns BrokerPeerInitRequest instance
     */
    public static create(properties?: IBrokerPeerInitRequest): BrokerPeerInitRequest;

    /**
     * Encodes the specified BrokerPeerInitRequest message. Does not implicitly {@link BrokerPeerInitRequest.verify|verify} messages.
     * @param message BrokerPeerInitRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IBrokerPeerInitRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified BrokerPeerInitRequest message, length delimited. Does not implicitly {@link BrokerPeerInitRequest.verify|verify} messages.
     * @param message BrokerPeerInitRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IBrokerPeerInitRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a BrokerPeerInitRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns BrokerPeerInitRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): BrokerPeerInitRequest;

    /**
     * Decodes a BrokerPeerInitRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns BrokerPeerInitRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): BrokerPeerInitRequest;

    /**
     * Verifies a BrokerPeerInitRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a BrokerPeerInitRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns BrokerPeerInitRequest
     */
    public static fromObject(object: { [k: string]: any }): BrokerPeerInitRequest;

    /**
     * Creates a plain object from a BrokerPeerInitRequest message. Also converts values to other types if specified.
     * @param message BrokerPeerInitRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: BrokerPeerInitRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this BrokerPeerInitRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a BrokerPeerDataRequest. */
export interface IBrokerPeerDataRequest {

    /** BrokerPeerDataRequest peerId */
    peerId?: (number|null);

    /** BrokerPeerDataRequest data */
    data?: (Uint8Array|null);
}

/** Represents a BrokerPeerDataRequest. */
export class BrokerPeerDataRequest implements IBrokerPeerDataRequest {

    /**
     * Constructs a new BrokerPeerDataRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: IBrokerPeerDataRequest);

    /** BrokerPeerDataRequest peerId. */
    public peerId: number;

    /** BrokerPeerDataRequest data. */
    public data: Uint8Array;

    /**
     * Creates a new BrokerPeerDataRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns BrokerPeerDataRequest instance
     */
    public static create(properties?: IBrokerPeerDataRequest): BrokerPeerDataRequest;

    /**
     * Encodes the specified BrokerPeerDataRequest message. Does not implicitly {@link BrokerPeerDataRequest.verify|verify} messages.
     * @param message BrokerPeerDataRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IBrokerPeerDataRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified BrokerPeerDataRequest message, length delimited. Does not implicitly {@link BrokerPeerDataRequest.verify|verify} messages.
     * @param message BrokerPeerDataRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IBrokerPeerDataRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a BrokerPeerDataRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns BrokerPeerDataRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): BrokerPeerDataRequest;

    /**
     * Decodes a BrokerPeerDataRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns BrokerPeerDataRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): BrokerPeerDataRequest;

    /**
     * Verifies a BrokerPeerDataRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a BrokerPeerDataRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns BrokerPeerDataRequest
     */
    public static fromObject(object: { [k: string]: any }): BrokerPeerDataRequest;

    /**
     * Creates a plain object from a BrokerPeerDataRequest message. Also converts values to other types if specified.
     * @param message BrokerPeerDataRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: BrokerPeerDataRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this BrokerPeerDataRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a BootstrapClient. */
export interface IBootstrapClient {

    /** BootstrapClient id */
    id?: (number|null);

    /** BootstrapClient websocketOptions */
    websocketOptions?: (IBootstrapClientWebSocketOptions|null);
}

/** Represents a BootstrapClient. */
export class BootstrapClient implements IBootstrapClient {

    /**
     * Constructs a new BootstrapClient.
     * @param [properties] Properties to set
     */
    constructor(properties?: IBootstrapClient);

    /** BootstrapClient id. */
    public id: number;

    /** BootstrapClient websocketOptions. */
    public websocketOptions?: (IBootstrapClientWebSocketOptions|null);

    /** BootstrapClient clientOptions. */
    public clientOptions?: "websocketOptions";

    /**
     * Creates a new BootstrapClient instance using the specified properties.
     * @param [properties] Properties to set
     * @returns BootstrapClient instance
     */
    public static create(properties?: IBootstrapClient): BootstrapClient;

    /**
     * Encodes the specified BootstrapClient message. Does not implicitly {@link BootstrapClient.verify|verify} messages.
     * @param message BootstrapClient message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IBootstrapClient, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified BootstrapClient message, length delimited. Does not implicitly {@link BootstrapClient.verify|verify} messages.
     * @param message BootstrapClient message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IBootstrapClient, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a BootstrapClient message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns BootstrapClient
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): BootstrapClient;

    /**
     * Decodes a BootstrapClient message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns BootstrapClient
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): BootstrapClient;

    /**
     * Verifies a BootstrapClient message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a BootstrapClient message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns BootstrapClient
     */
    public static fromObject(object: { [k: string]: any }): BootstrapClient;

    /**
     * Creates a plain object from a BootstrapClient message. Also converts values to other types if specified.
     * @param message BootstrapClient
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: BootstrapClient, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this BootstrapClient to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a BootstrapClientWebSocketOptions. */
export interface IBootstrapClientWebSocketOptions {

    /** BootstrapClientWebSocketOptions url */
    url?: (string|null);

    /** BootstrapClientWebSocketOptions insecureSkipVerifyTls */
    insecureSkipVerifyTls?: (boolean|null);
}

/** Represents a BootstrapClientWebSocketOptions. */
export class BootstrapClientWebSocketOptions implements IBootstrapClientWebSocketOptions {

    /**
     * Constructs a new BootstrapClientWebSocketOptions.
     * @param [properties] Properties to set
     */
    constructor(properties?: IBootstrapClientWebSocketOptions);

    /** BootstrapClientWebSocketOptions url. */
    public url: string;

    /** BootstrapClientWebSocketOptions insecureSkipVerifyTls. */
    public insecureSkipVerifyTls: boolean;

    /**
     * Creates a new BootstrapClientWebSocketOptions instance using the specified properties.
     * @param [properties] Properties to set
     * @returns BootstrapClientWebSocketOptions instance
     */
    public static create(properties?: IBootstrapClientWebSocketOptions): BootstrapClientWebSocketOptions;

    /**
     * Encodes the specified BootstrapClientWebSocketOptions message. Does not implicitly {@link BootstrapClientWebSocketOptions.verify|verify} messages.
     * @param message BootstrapClientWebSocketOptions message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IBootstrapClientWebSocketOptions, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified BootstrapClientWebSocketOptions message, length delimited. Does not implicitly {@link BootstrapClientWebSocketOptions.verify|verify} messages.
     * @param message BootstrapClientWebSocketOptions message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IBootstrapClientWebSocketOptions, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a BootstrapClientWebSocketOptions message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns BootstrapClientWebSocketOptions
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): BootstrapClientWebSocketOptions;

    /**
     * Decodes a BootstrapClientWebSocketOptions message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns BootstrapClientWebSocketOptions
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): BootstrapClientWebSocketOptions;

    /**
     * Verifies a BootstrapClientWebSocketOptions message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a BootstrapClientWebSocketOptions message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns BootstrapClientWebSocketOptions
     */
    public static fromObject(object: { [k: string]: any }): BootstrapClientWebSocketOptions;

    /**
     * Creates a plain object from a BootstrapClientWebSocketOptions message. Also converts values to other types if specified.
     * @param message BootstrapClientWebSocketOptions
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: BootstrapClientWebSocketOptions, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this BootstrapClientWebSocketOptions to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a CreateBootstrapClientRequest. */
export interface ICreateBootstrapClientRequest {

    /** CreateBootstrapClientRequest websocketOptions */
    websocketOptions?: (IBootstrapClientWebSocketOptions|null);
}

/** Represents a CreateBootstrapClientRequest. */
export class CreateBootstrapClientRequest implements ICreateBootstrapClientRequest {

    /**
     * Constructs a new CreateBootstrapClientRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: ICreateBootstrapClientRequest);

    /** CreateBootstrapClientRequest websocketOptions. */
    public websocketOptions?: (IBootstrapClientWebSocketOptions|null);

    /** CreateBootstrapClientRequest clientOptions. */
    public clientOptions?: "websocketOptions";

    /**
     * Creates a new CreateBootstrapClientRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns CreateBootstrapClientRequest instance
     */
    public static create(properties?: ICreateBootstrapClientRequest): CreateBootstrapClientRequest;

    /**
     * Encodes the specified CreateBootstrapClientRequest message. Does not implicitly {@link CreateBootstrapClientRequest.verify|verify} messages.
     * @param message CreateBootstrapClientRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: ICreateBootstrapClientRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified CreateBootstrapClientRequest message, length delimited. Does not implicitly {@link CreateBootstrapClientRequest.verify|verify} messages.
     * @param message CreateBootstrapClientRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: ICreateBootstrapClientRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a CreateBootstrapClientRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns CreateBootstrapClientRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): CreateBootstrapClientRequest;

    /**
     * Decodes a CreateBootstrapClientRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns CreateBootstrapClientRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): CreateBootstrapClientRequest;

    /**
     * Verifies a CreateBootstrapClientRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a CreateBootstrapClientRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns CreateBootstrapClientRequest
     */
    public static fromObject(object: { [k: string]: any }): CreateBootstrapClientRequest;

    /**
     * Creates a plain object from a CreateBootstrapClientRequest message. Also converts values to other types if specified.
     * @param message CreateBootstrapClientRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: CreateBootstrapClientRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this CreateBootstrapClientRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a CreateBootstrapClientResponse. */
export interface ICreateBootstrapClientResponse {

    /** CreateBootstrapClientResponse bootstrapClient */
    bootstrapClient?: (IBootstrapClient|null);
}

/** Represents a CreateBootstrapClientResponse. */
export class CreateBootstrapClientResponse implements ICreateBootstrapClientResponse {

    /**
     * Constructs a new CreateBootstrapClientResponse.
     * @param [properties] Properties to set
     */
    constructor(properties?: ICreateBootstrapClientResponse);

    /** CreateBootstrapClientResponse bootstrapClient. */
    public bootstrapClient?: (IBootstrapClient|null);

    /**
     * Creates a new CreateBootstrapClientResponse instance using the specified properties.
     * @param [properties] Properties to set
     * @returns CreateBootstrapClientResponse instance
     */
    public static create(properties?: ICreateBootstrapClientResponse): CreateBootstrapClientResponse;

    /**
     * Encodes the specified CreateBootstrapClientResponse message. Does not implicitly {@link CreateBootstrapClientResponse.verify|verify} messages.
     * @param message CreateBootstrapClientResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: ICreateBootstrapClientResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified CreateBootstrapClientResponse message, length delimited. Does not implicitly {@link CreateBootstrapClientResponse.verify|verify} messages.
     * @param message CreateBootstrapClientResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: ICreateBootstrapClientResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a CreateBootstrapClientResponse message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns CreateBootstrapClientResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): CreateBootstrapClientResponse;

    /**
     * Decodes a CreateBootstrapClientResponse message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns CreateBootstrapClientResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): CreateBootstrapClientResponse;

    /**
     * Verifies a CreateBootstrapClientResponse message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a CreateBootstrapClientResponse message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns CreateBootstrapClientResponse
     */
    public static fromObject(object: { [k: string]: any }): CreateBootstrapClientResponse;

    /**
     * Creates a plain object from a CreateBootstrapClientResponse message. Also converts values to other types if specified.
     * @param message CreateBootstrapClientResponse
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: CreateBootstrapClientResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this CreateBootstrapClientResponse to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of an UpdateBootstrapClientRequest. */
export interface IUpdateBootstrapClientRequest {

    /** UpdateBootstrapClientRequest id */
    id?: (number|null);

    /** UpdateBootstrapClientRequest websocketOptions */
    websocketOptions?: (IBootstrapClientWebSocketOptions|null);
}

/** Represents an UpdateBootstrapClientRequest. */
export class UpdateBootstrapClientRequest implements IUpdateBootstrapClientRequest {

    /**
     * Constructs a new UpdateBootstrapClientRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: IUpdateBootstrapClientRequest);

    /** UpdateBootstrapClientRequest id. */
    public id: number;

    /** UpdateBootstrapClientRequest websocketOptions. */
    public websocketOptions?: (IBootstrapClientWebSocketOptions|null);

    /** UpdateBootstrapClientRequest clientOptions. */
    public clientOptions?: "websocketOptions";

    /**
     * Creates a new UpdateBootstrapClientRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns UpdateBootstrapClientRequest instance
     */
    public static create(properties?: IUpdateBootstrapClientRequest): UpdateBootstrapClientRequest;

    /**
     * Encodes the specified UpdateBootstrapClientRequest message. Does not implicitly {@link UpdateBootstrapClientRequest.verify|verify} messages.
     * @param message UpdateBootstrapClientRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IUpdateBootstrapClientRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified UpdateBootstrapClientRequest message, length delimited. Does not implicitly {@link UpdateBootstrapClientRequest.verify|verify} messages.
     * @param message UpdateBootstrapClientRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IUpdateBootstrapClientRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes an UpdateBootstrapClientRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns UpdateBootstrapClientRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): UpdateBootstrapClientRequest;

    /**
     * Decodes an UpdateBootstrapClientRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns UpdateBootstrapClientRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): UpdateBootstrapClientRequest;

    /**
     * Verifies an UpdateBootstrapClientRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates an UpdateBootstrapClientRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns UpdateBootstrapClientRequest
     */
    public static fromObject(object: { [k: string]: any }): UpdateBootstrapClientRequest;

    /**
     * Creates a plain object from an UpdateBootstrapClientRequest message. Also converts values to other types if specified.
     * @param message UpdateBootstrapClientRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: UpdateBootstrapClientRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this UpdateBootstrapClientRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of an UpdateBootstrapClientResponse. */
export interface IUpdateBootstrapClientResponse {

    /** UpdateBootstrapClientResponse bootstrapClient */
    bootstrapClient?: (IBootstrapClient|null);
}

/** Represents an UpdateBootstrapClientResponse. */
export class UpdateBootstrapClientResponse implements IUpdateBootstrapClientResponse {

    /**
     * Constructs a new UpdateBootstrapClientResponse.
     * @param [properties] Properties to set
     */
    constructor(properties?: IUpdateBootstrapClientResponse);

    /** UpdateBootstrapClientResponse bootstrapClient. */
    public bootstrapClient?: (IBootstrapClient|null);

    /**
     * Creates a new UpdateBootstrapClientResponse instance using the specified properties.
     * @param [properties] Properties to set
     * @returns UpdateBootstrapClientResponse instance
     */
    public static create(properties?: IUpdateBootstrapClientResponse): UpdateBootstrapClientResponse;

    /**
     * Encodes the specified UpdateBootstrapClientResponse message. Does not implicitly {@link UpdateBootstrapClientResponse.verify|verify} messages.
     * @param message UpdateBootstrapClientResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IUpdateBootstrapClientResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified UpdateBootstrapClientResponse message, length delimited. Does not implicitly {@link UpdateBootstrapClientResponse.verify|verify} messages.
     * @param message UpdateBootstrapClientResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IUpdateBootstrapClientResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes an UpdateBootstrapClientResponse message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns UpdateBootstrapClientResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): UpdateBootstrapClientResponse;

    /**
     * Decodes an UpdateBootstrapClientResponse message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns UpdateBootstrapClientResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): UpdateBootstrapClientResponse;

    /**
     * Verifies an UpdateBootstrapClientResponse message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates an UpdateBootstrapClientResponse message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns UpdateBootstrapClientResponse
     */
    public static fromObject(object: { [k: string]: any }): UpdateBootstrapClientResponse;

    /**
     * Creates a plain object from an UpdateBootstrapClientResponse message. Also converts values to other types if specified.
     * @param message UpdateBootstrapClientResponse
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: UpdateBootstrapClientResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this UpdateBootstrapClientResponse to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a DeleteBootstrapClientRequest. */
export interface IDeleteBootstrapClientRequest {

    /** DeleteBootstrapClientRequest id */
    id?: (number|null);
}

/** Represents a DeleteBootstrapClientRequest. */
export class DeleteBootstrapClientRequest implements IDeleteBootstrapClientRequest {

    /**
     * Constructs a new DeleteBootstrapClientRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: IDeleteBootstrapClientRequest);

    /** DeleteBootstrapClientRequest id. */
    public id: number;

    /**
     * Creates a new DeleteBootstrapClientRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns DeleteBootstrapClientRequest instance
     */
    public static create(properties?: IDeleteBootstrapClientRequest): DeleteBootstrapClientRequest;

    /**
     * Encodes the specified DeleteBootstrapClientRequest message. Does not implicitly {@link DeleteBootstrapClientRequest.verify|verify} messages.
     * @param message DeleteBootstrapClientRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IDeleteBootstrapClientRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified DeleteBootstrapClientRequest message, length delimited. Does not implicitly {@link DeleteBootstrapClientRequest.verify|verify} messages.
     * @param message DeleteBootstrapClientRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IDeleteBootstrapClientRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a DeleteBootstrapClientRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns DeleteBootstrapClientRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): DeleteBootstrapClientRequest;

    /**
     * Decodes a DeleteBootstrapClientRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns DeleteBootstrapClientRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): DeleteBootstrapClientRequest;

    /**
     * Verifies a DeleteBootstrapClientRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a DeleteBootstrapClientRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns DeleteBootstrapClientRequest
     */
    public static fromObject(object: { [k: string]: any }): DeleteBootstrapClientRequest;

    /**
     * Creates a plain object from a DeleteBootstrapClientRequest message. Also converts values to other types if specified.
     * @param message DeleteBootstrapClientRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: DeleteBootstrapClientRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this DeleteBootstrapClientRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a DeleteBootstrapClientResponse. */
export interface IDeleteBootstrapClientResponse {
}

/** Represents a DeleteBootstrapClientResponse. */
export class DeleteBootstrapClientResponse implements IDeleteBootstrapClientResponse {

    /**
     * Constructs a new DeleteBootstrapClientResponse.
     * @param [properties] Properties to set
     */
    constructor(properties?: IDeleteBootstrapClientResponse);

    /**
     * Creates a new DeleteBootstrapClientResponse instance using the specified properties.
     * @param [properties] Properties to set
     * @returns DeleteBootstrapClientResponse instance
     */
    public static create(properties?: IDeleteBootstrapClientResponse): DeleteBootstrapClientResponse;

    /**
     * Encodes the specified DeleteBootstrapClientResponse message. Does not implicitly {@link DeleteBootstrapClientResponse.verify|verify} messages.
     * @param message DeleteBootstrapClientResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IDeleteBootstrapClientResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified DeleteBootstrapClientResponse message, length delimited. Does not implicitly {@link DeleteBootstrapClientResponse.verify|verify} messages.
     * @param message DeleteBootstrapClientResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IDeleteBootstrapClientResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a DeleteBootstrapClientResponse message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns DeleteBootstrapClientResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): DeleteBootstrapClientResponse;

    /**
     * Decodes a DeleteBootstrapClientResponse message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns DeleteBootstrapClientResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): DeleteBootstrapClientResponse;

    /**
     * Verifies a DeleteBootstrapClientResponse message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a DeleteBootstrapClientResponse message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns DeleteBootstrapClientResponse
     */
    public static fromObject(object: { [k: string]: any }): DeleteBootstrapClientResponse;

    /**
     * Creates a plain object from a DeleteBootstrapClientResponse message. Also converts values to other types if specified.
     * @param message DeleteBootstrapClientResponse
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: DeleteBootstrapClientResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this DeleteBootstrapClientResponse to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a GetBootstrapClientRequest. */
export interface IGetBootstrapClientRequest {

    /** GetBootstrapClientRequest id */
    id?: (number|null);
}

/** Represents a GetBootstrapClientRequest. */
export class GetBootstrapClientRequest implements IGetBootstrapClientRequest {

    /**
     * Constructs a new GetBootstrapClientRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: IGetBootstrapClientRequest);

    /** GetBootstrapClientRequest id. */
    public id: number;

    /**
     * Creates a new GetBootstrapClientRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns GetBootstrapClientRequest instance
     */
    public static create(properties?: IGetBootstrapClientRequest): GetBootstrapClientRequest;

    /**
     * Encodes the specified GetBootstrapClientRequest message. Does not implicitly {@link GetBootstrapClientRequest.verify|verify} messages.
     * @param message GetBootstrapClientRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IGetBootstrapClientRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified GetBootstrapClientRequest message, length delimited. Does not implicitly {@link GetBootstrapClientRequest.verify|verify} messages.
     * @param message GetBootstrapClientRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IGetBootstrapClientRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a GetBootstrapClientRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns GetBootstrapClientRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): GetBootstrapClientRequest;

    /**
     * Decodes a GetBootstrapClientRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns GetBootstrapClientRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): GetBootstrapClientRequest;

    /**
     * Verifies a GetBootstrapClientRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a GetBootstrapClientRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns GetBootstrapClientRequest
     */
    public static fromObject(object: { [k: string]: any }): GetBootstrapClientRequest;

    /**
     * Creates a plain object from a GetBootstrapClientRequest message. Also converts values to other types if specified.
     * @param message GetBootstrapClientRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: GetBootstrapClientRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this GetBootstrapClientRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a GetBootstrapClientResponse. */
export interface IGetBootstrapClientResponse {

    /** GetBootstrapClientResponse bootstrapClient */
    bootstrapClient?: (IBootstrapClient|null);
}

/** Represents a GetBootstrapClientResponse. */
export class GetBootstrapClientResponse implements IGetBootstrapClientResponse {

    /**
     * Constructs a new GetBootstrapClientResponse.
     * @param [properties] Properties to set
     */
    constructor(properties?: IGetBootstrapClientResponse);

    /** GetBootstrapClientResponse bootstrapClient. */
    public bootstrapClient?: (IBootstrapClient|null);

    /**
     * Creates a new GetBootstrapClientResponse instance using the specified properties.
     * @param [properties] Properties to set
     * @returns GetBootstrapClientResponse instance
     */
    public static create(properties?: IGetBootstrapClientResponse): GetBootstrapClientResponse;

    /**
     * Encodes the specified GetBootstrapClientResponse message. Does not implicitly {@link GetBootstrapClientResponse.verify|verify} messages.
     * @param message GetBootstrapClientResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IGetBootstrapClientResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified GetBootstrapClientResponse message, length delimited. Does not implicitly {@link GetBootstrapClientResponse.verify|verify} messages.
     * @param message GetBootstrapClientResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IGetBootstrapClientResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a GetBootstrapClientResponse message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns GetBootstrapClientResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): GetBootstrapClientResponse;

    /**
     * Decodes a GetBootstrapClientResponse message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns GetBootstrapClientResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): GetBootstrapClientResponse;

    /**
     * Verifies a GetBootstrapClientResponse message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a GetBootstrapClientResponse message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns GetBootstrapClientResponse
     */
    public static fromObject(object: { [k: string]: any }): GetBootstrapClientResponse;

    /**
     * Creates a plain object from a GetBootstrapClientResponse message. Also converts values to other types if specified.
     * @param message GetBootstrapClientResponse
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: GetBootstrapClientResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this GetBootstrapClientResponse to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a GetBootstrapClientsRequest. */
export interface IGetBootstrapClientsRequest {
}

/** Represents a GetBootstrapClientsRequest. */
export class GetBootstrapClientsRequest implements IGetBootstrapClientsRequest {

    /**
     * Constructs a new GetBootstrapClientsRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: IGetBootstrapClientsRequest);

    /**
     * Creates a new GetBootstrapClientsRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns GetBootstrapClientsRequest instance
     */
    public static create(properties?: IGetBootstrapClientsRequest): GetBootstrapClientsRequest;

    /**
     * Encodes the specified GetBootstrapClientsRequest message. Does not implicitly {@link GetBootstrapClientsRequest.verify|verify} messages.
     * @param message GetBootstrapClientsRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IGetBootstrapClientsRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified GetBootstrapClientsRequest message, length delimited. Does not implicitly {@link GetBootstrapClientsRequest.verify|verify} messages.
     * @param message GetBootstrapClientsRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IGetBootstrapClientsRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a GetBootstrapClientsRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns GetBootstrapClientsRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): GetBootstrapClientsRequest;

    /**
     * Decodes a GetBootstrapClientsRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns GetBootstrapClientsRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): GetBootstrapClientsRequest;

    /**
     * Verifies a GetBootstrapClientsRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a GetBootstrapClientsRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns GetBootstrapClientsRequest
     */
    public static fromObject(object: { [k: string]: any }): GetBootstrapClientsRequest;

    /**
     * Creates a plain object from a GetBootstrapClientsRequest message. Also converts values to other types if specified.
     * @param message GetBootstrapClientsRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: GetBootstrapClientsRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this GetBootstrapClientsRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a GetBootstrapClientsResponse. */
export interface IGetBootstrapClientsResponse {

    /** GetBootstrapClientsResponse bootstrapClients */
    bootstrapClients?: (IBootstrapClient[]|null);
}

/** Represents a GetBootstrapClientsResponse. */
export class GetBootstrapClientsResponse implements IGetBootstrapClientsResponse {

    /**
     * Constructs a new GetBootstrapClientsResponse.
     * @param [properties] Properties to set
     */
    constructor(properties?: IGetBootstrapClientsResponse);

    /** GetBootstrapClientsResponse bootstrapClients. */
    public bootstrapClients: IBootstrapClient[];

    /**
     * Creates a new GetBootstrapClientsResponse instance using the specified properties.
     * @param [properties] Properties to set
     * @returns GetBootstrapClientsResponse instance
     */
    public static create(properties?: IGetBootstrapClientsResponse): GetBootstrapClientsResponse;

    /**
     * Encodes the specified GetBootstrapClientsResponse message. Does not implicitly {@link GetBootstrapClientsResponse.verify|verify} messages.
     * @param message GetBootstrapClientsResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IGetBootstrapClientsResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified GetBootstrapClientsResponse message, length delimited. Does not implicitly {@link GetBootstrapClientsResponse.verify|verify} messages.
     * @param message GetBootstrapClientsResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IGetBootstrapClientsResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a GetBootstrapClientsResponse message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns GetBootstrapClientsResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): GetBootstrapClientsResponse;

    /**
     * Decodes a GetBootstrapClientsResponse message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns GetBootstrapClientsResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): GetBootstrapClientsResponse;

    /**
     * Verifies a GetBootstrapClientsResponse message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a GetBootstrapClientsResponse message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns GetBootstrapClientsResponse
     */
    public static fromObject(object: { [k: string]: any }): GetBootstrapClientsResponse;

    /**
     * Creates a plain object from a GetBootstrapClientsResponse message. Also converts values to other types if specified.
     * @param message GetBootstrapClientsResponse
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: GetBootstrapClientsResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this GetBootstrapClientsResponse to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a GetBootstrapPeersRequest. */
export interface IGetBootstrapPeersRequest {
}

/** Represents a GetBootstrapPeersRequest. */
export class GetBootstrapPeersRequest implements IGetBootstrapPeersRequest {

    /**
     * Constructs a new GetBootstrapPeersRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: IGetBootstrapPeersRequest);

    /**
     * Creates a new GetBootstrapPeersRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns GetBootstrapPeersRequest instance
     */
    public static create(properties?: IGetBootstrapPeersRequest): GetBootstrapPeersRequest;

    /**
     * Encodes the specified GetBootstrapPeersRequest message. Does not implicitly {@link GetBootstrapPeersRequest.verify|verify} messages.
     * @param message GetBootstrapPeersRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IGetBootstrapPeersRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified GetBootstrapPeersRequest message, length delimited. Does not implicitly {@link GetBootstrapPeersRequest.verify|verify} messages.
     * @param message GetBootstrapPeersRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IGetBootstrapPeersRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a GetBootstrapPeersRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns GetBootstrapPeersRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): GetBootstrapPeersRequest;

    /**
     * Decodes a GetBootstrapPeersRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns GetBootstrapPeersRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): GetBootstrapPeersRequest;

    /**
     * Verifies a GetBootstrapPeersRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a GetBootstrapPeersRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns GetBootstrapPeersRequest
     */
    public static fromObject(object: { [k: string]: any }): GetBootstrapPeersRequest;

    /**
     * Creates a plain object from a GetBootstrapPeersRequest message. Also converts values to other types if specified.
     * @param message GetBootstrapPeersRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: GetBootstrapPeersRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this GetBootstrapPeersRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a GetBootstrapPeersResponse. */
export interface IGetBootstrapPeersResponse {

    /** GetBootstrapPeersResponse peers */
    peers?: (IBootstrapPeer[]|null);
}

/** Represents a GetBootstrapPeersResponse. */
export class GetBootstrapPeersResponse implements IGetBootstrapPeersResponse {

    /**
     * Constructs a new GetBootstrapPeersResponse.
     * @param [properties] Properties to set
     */
    constructor(properties?: IGetBootstrapPeersResponse);

    /** GetBootstrapPeersResponse peers. */
    public peers: IBootstrapPeer[];

    /**
     * Creates a new GetBootstrapPeersResponse instance using the specified properties.
     * @param [properties] Properties to set
     * @returns GetBootstrapPeersResponse instance
     */
    public static create(properties?: IGetBootstrapPeersResponse): GetBootstrapPeersResponse;

    /**
     * Encodes the specified GetBootstrapPeersResponse message. Does not implicitly {@link GetBootstrapPeersResponse.verify|verify} messages.
     * @param message GetBootstrapPeersResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IGetBootstrapPeersResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified GetBootstrapPeersResponse message, length delimited. Does not implicitly {@link GetBootstrapPeersResponse.verify|verify} messages.
     * @param message GetBootstrapPeersResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IGetBootstrapPeersResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a GetBootstrapPeersResponse message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns GetBootstrapPeersResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): GetBootstrapPeersResponse;

    /**
     * Decodes a GetBootstrapPeersResponse message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns GetBootstrapPeersResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): GetBootstrapPeersResponse;

    /**
     * Verifies a GetBootstrapPeersResponse message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a GetBootstrapPeersResponse message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns GetBootstrapPeersResponse
     */
    public static fromObject(object: { [k: string]: any }): GetBootstrapPeersResponse;

    /**
     * Creates a plain object from a GetBootstrapPeersResponse message. Also converts values to other types if specified.
     * @param message GetBootstrapPeersResponse
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: GetBootstrapPeersResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this GetBootstrapPeersResponse to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a BootstrapPeer. */
export interface IBootstrapPeer {

    /** BootstrapPeer hostId */
    hostId?: (Uint8Array|null);

    /** BootstrapPeer label */
    label?: (string|null);
}

/** Represents a BootstrapPeer. */
export class BootstrapPeer implements IBootstrapPeer {

    /**
     * Constructs a new BootstrapPeer.
     * @param [properties] Properties to set
     */
    constructor(properties?: IBootstrapPeer);

    /** BootstrapPeer hostId. */
    public hostId: Uint8Array;

    /** BootstrapPeer label. */
    public label: string;

    /**
     * Creates a new BootstrapPeer instance using the specified properties.
     * @param [properties] Properties to set
     * @returns BootstrapPeer instance
     */
    public static create(properties?: IBootstrapPeer): BootstrapPeer;

    /**
     * Encodes the specified BootstrapPeer message. Does not implicitly {@link BootstrapPeer.verify|verify} messages.
     * @param message BootstrapPeer message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IBootstrapPeer, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified BootstrapPeer message, length delimited. Does not implicitly {@link BootstrapPeer.verify|verify} messages.
     * @param message BootstrapPeer message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IBootstrapPeer, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a BootstrapPeer message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns BootstrapPeer
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): BootstrapPeer;

    /**
     * Decodes a BootstrapPeer message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns BootstrapPeer
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): BootstrapPeer;

    /**
     * Verifies a BootstrapPeer message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a BootstrapPeer message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns BootstrapPeer
     */
    public static fromObject(object: { [k: string]: any }): BootstrapPeer;

    /**
     * Creates a plain object from a BootstrapPeer message. Also converts values to other types if specified.
     * @param message BootstrapPeer
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: BootstrapPeer, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this BootstrapPeer to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a BootstrapServiceMessage. */
export interface IBootstrapServiceMessage {

    /** BootstrapServiceMessage brokerOffer */
    brokerOffer?: (BootstrapServiceMessage.IBrokerOffer|null);

    /** BootstrapServiceMessage publishRequest */
    publishRequest?: (BootstrapServiceMessage.IPublishRequest|null);

    /** BootstrapServiceMessage publishResponse */
    publishResponse?: (BootstrapServiceMessage.IPublishResponse|null);
}

/** Represents a BootstrapServiceMessage. */
export class BootstrapServiceMessage implements IBootstrapServiceMessage {

    /**
     * Constructs a new BootstrapServiceMessage.
     * @param [properties] Properties to set
     */
    constructor(properties?: IBootstrapServiceMessage);

    /** BootstrapServiceMessage brokerOffer. */
    public brokerOffer?: (BootstrapServiceMessage.IBrokerOffer|null);

    /** BootstrapServiceMessage publishRequest. */
    public publishRequest?: (BootstrapServiceMessage.IPublishRequest|null);

    /** BootstrapServiceMessage publishResponse. */
    public publishResponse?: (BootstrapServiceMessage.IPublishResponse|null);

    /** BootstrapServiceMessage body. */
    public body?: ("brokerOffer"|"publishRequest"|"publishResponse");

    /**
     * Creates a new BootstrapServiceMessage instance using the specified properties.
     * @param [properties] Properties to set
     * @returns BootstrapServiceMessage instance
     */
    public static create(properties?: IBootstrapServiceMessage): BootstrapServiceMessage;

    /**
     * Encodes the specified BootstrapServiceMessage message. Does not implicitly {@link BootstrapServiceMessage.verify|verify} messages.
     * @param message BootstrapServiceMessage message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IBootstrapServiceMessage, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified BootstrapServiceMessage message, length delimited. Does not implicitly {@link BootstrapServiceMessage.verify|verify} messages.
     * @param message BootstrapServiceMessage message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IBootstrapServiceMessage, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a BootstrapServiceMessage message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns BootstrapServiceMessage
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): BootstrapServiceMessage;

    /**
     * Decodes a BootstrapServiceMessage message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns BootstrapServiceMessage
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): BootstrapServiceMessage;

    /**
     * Verifies a BootstrapServiceMessage message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a BootstrapServiceMessage message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns BootstrapServiceMessage
     */
    public static fromObject(object: { [k: string]: any }): BootstrapServiceMessage;

    /**
     * Creates a plain object from a BootstrapServiceMessage message. Also converts values to other types if specified.
     * @param message BootstrapServiceMessage
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: BootstrapServiceMessage, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this BootstrapServiceMessage to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

export namespace BootstrapServiceMessage {

    /** Properties of a BrokerOffer. */
    interface IBrokerOffer {
    }

    /** Represents a BrokerOffer. */
    class BrokerOffer implements IBrokerOffer {

        /**
         * Constructs a new BrokerOffer.
         * @param [properties] Properties to set
         */
        constructor(properties?: BootstrapServiceMessage.IBrokerOffer);

        /**
         * Creates a new BrokerOffer instance using the specified properties.
         * @param [properties] Properties to set
         * @returns BrokerOffer instance
         */
        public static create(properties?: BootstrapServiceMessage.IBrokerOffer): BootstrapServiceMessage.BrokerOffer;

        /**
         * Encodes the specified BrokerOffer message. Does not implicitly {@link BootstrapServiceMessage.BrokerOffer.verify|verify} messages.
         * @param message BrokerOffer message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: BootstrapServiceMessage.IBrokerOffer, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified BrokerOffer message, length delimited. Does not implicitly {@link BootstrapServiceMessage.BrokerOffer.verify|verify} messages.
         * @param message BrokerOffer message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: BootstrapServiceMessage.IBrokerOffer, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a BrokerOffer message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns BrokerOffer
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): BootstrapServiceMessage.BrokerOffer;

        /**
         * Decodes a BrokerOffer message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns BrokerOffer
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): BootstrapServiceMessage.BrokerOffer;

        /**
         * Verifies a BrokerOffer message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a BrokerOffer message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns BrokerOffer
         */
        public static fromObject(object: { [k: string]: any }): BootstrapServiceMessage.BrokerOffer;

        /**
         * Creates a plain object from a BrokerOffer message. Also converts values to other types if specified.
         * @param message BrokerOffer
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: BootstrapServiceMessage.BrokerOffer, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this BrokerOffer to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }

    /** Properties of a PublishRequest. */
    interface IPublishRequest {

        /** PublishRequest name */
        name?: (string|null);

        /** PublishRequest certificate */
        certificate?: (ICertificate|null);
    }

    /** Represents a PublishRequest. */
    class PublishRequest implements IPublishRequest {

        /**
         * Constructs a new PublishRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: BootstrapServiceMessage.IPublishRequest);

        /** PublishRequest name. */
        public name: string;

        /** PublishRequest certificate. */
        public certificate?: (ICertificate|null);

        /**
         * Creates a new PublishRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns PublishRequest instance
         */
        public static create(properties?: BootstrapServiceMessage.IPublishRequest): BootstrapServiceMessage.PublishRequest;

        /**
         * Encodes the specified PublishRequest message. Does not implicitly {@link BootstrapServiceMessage.PublishRequest.verify|verify} messages.
         * @param message PublishRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: BootstrapServiceMessage.IPublishRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified PublishRequest message, length delimited. Does not implicitly {@link BootstrapServiceMessage.PublishRequest.verify|verify} messages.
         * @param message PublishRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: BootstrapServiceMessage.IPublishRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a PublishRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns PublishRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): BootstrapServiceMessage.PublishRequest;

        /**
         * Decodes a PublishRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns PublishRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): BootstrapServiceMessage.PublishRequest;

        /**
         * Verifies a PublishRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a PublishRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns PublishRequest
         */
        public static fromObject(object: { [k: string]: any }): BootstrapServiceMessage.PublishRequest;

        /**
         * Creates a plain object from a PublishRequest message. Also converts values to other types if specified.
         * @param message PublishRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: BootstrapServiceMessage.PublishRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this PublishRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }

    /** Properties of a PublishResponse. */
    interface IPublishResponse {

        /** PublishResponse error */
        error?: (string|null);
    }

    /** Represents a PublishResponse. */
    class PublishResponse implements IPublishResponse {

        /**
         * Constructs a new PublishResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: BootstrapServiceMessage.IPublishResponse);

        /** PublishResponse error. */
        public error: string;

        /** PublishResponse body. */
        public body?: "error";

        /**
         * Creates a new PublishResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns PublishResponse instance
         */
        public static create(properties?: BootstrapServiceMessage.IPublishResponse): BootstrapServiceMessage.PublishResponse;

        /**
         * Encodes the specified PublishResponse message. Does not implicitly {@link BootstrapServiceMessage.PublishResponse.verify|verify} messages.
         * @param message PublishResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: BootstrapServiceMessage.IPublishResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified PublishResponse message, length delimited. Does not implicitly {@link BootstrapServiceMessage.PublishResponse.verify|verify} messages.
         * @param message PublishResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: BootstrapServiceMessage.IPublishResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a PublishResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns PublishResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): BootstrapServiceMessage.PublishResponse;

        /**
         * Decodes a PublishResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns PublishResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): BootstrapServiceMessage.PublishResponse;

        /**
         * Verifies a PublishResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a PublishResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns PublishResponse
         */
        public static fromObject(object: { [k: string]: any }): BootstrapServiceMessage.PublishResponse;

        /**
         * Creates a plain object from a PublishResponse message. Also converts values to other types if specified.
         * @param message PublishResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: BootstrapServiceMessage.PublishResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this PublishResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }
}

/** Properties of a PublishNetworkToBootstrapPeerRequest. */
export interface IPublishNetworkToBootstrapPeerRequest {

    /** PublishNetworkToBootstrapPeerRequest hostId */
    hostId?: (Uint8Array|null);

    /** PublishNetworkToBootstrapPeerRequest network */
    network?: (INetwork|null);
}

/** Represents a PublishNetworkToBootstrapPeerRequest. */
export class PublishNetworkToBootstrapPeerRequest implements IPublishNetworkToBootstrapPeerRequest {

    /**
     * Constructs a new PublishNetworkToBootstrapPeerRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: IPublishNetworkToBootstrapPeerRequest);

    /** PublishNetworkToBootstrapPeerRequest hostId. */
    public hostId: Uint8Array;

    /** PublishNetworkToBootstrapPeerRequest network. */
    public network?: (INetwork|null);

    /**
     * Creates a new PublishNetworkToBootstrapPeerRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns PublishNetworkToBootstrapPeerRequest instance
     */
    public static create(properties?: IPublishNetworkToBootstrapPeerRequest): PublishNetworkToBootstrapPeerRequest;

    /**
     * Encodes the specified PublishNetworkToBootstrapPeerRequest message. Does not implicitly {@link PublishNetworkToBootstrapPeerRequest.verify|verify} messages.
     * @param message PublishNetworkToBootstrapPeerRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IPublishNetworkToBootstrapPeerRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified PublishNetworkToBootstrapPeerRequest message, length delimited. Does not implicitly {@link PublishNetworkToBootstrapPeerRequest.verify|verify} messages.
     * @param message PublishNetworkToBootstrapPeerRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IPublishNetworkToBootstrapPeerRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a PublishNetworkToBootstrapPeerRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns PublishNetworkToBootstrapPeerRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): PublishNetworkToBootstrapPeerRequest;

    /**
     * Decodes a PublishNetworkToBootstrapPeerRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns PublishNetworkToBootstrapPeerRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): PublishNetworkToBootstrapPeerRequest;

    /**
     * Verifies a PublishNetworkToBootstrapPeerRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a PublishNetworkToBootstrapPeerRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns PublishNetworkToBootstrapPeerRequest
     */
    public static fromObject(object: { [k: string]: any }): PublishNetworkToBootstrapPeerRequest;

    /**
     * Creates a plain object from a PublishNetworkToBootstrapPeerRequest message. Also converts values to other types if specified.
     * @param message PublishNetworkToBootstrapPeerRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: PublishNetworkToBootstrapPeerRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this PublishNetworkToBootstrapPeerRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a PublishNetworkToBootstrapPeerResponse. */
export interface IPublishNetworkToBootstrapPeerResponse {
}

/** Represents a PublishNetworkToBootstrapPeerResponse. */
export class PublishNetworkToBootstrapPeerResponse implements IPublishNetworkToBootstrapPeerResponse {

    /**
     * Constructs a new PublishNetworkToBootstrapPeerResponse.
     * @param [properties] Properties to set
     */
    constructor(properties?: IPublishNetworkToBootstrapPeerResponse);

    /**
     * Creates a new PublishNetworkToBootstrapPeerResponse instance using the specified properties.
     * @param [properties] Properties to set
     * @returns PublishNetworkToBootstrapPeerResponse instance
     */
    public static create(properties?: IPublishNetworkToBootstrapPeerResponse): PublishNetworkToBootstrapPeerResponse;

    /**
     * Encodes the specified PublishNetworkToBootstrapPeerResponse message. Does not implicitly {@link PublishNetworkToBootstrapPeerResponse.verify|verify} messages.
     * @param message PublishNetworkToBootstrapPeerResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IPublishNetworkToBootstrapPeerResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified PublishNetworkToBootstrapPeerResponse message, length delimited. Does not implicitly {@link PublishNetworkToBootstrapPeerResponse.verify|verify} messages.
     * @param message PublishNetworkToBootstrapPeerResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IPublishNetworkToBootstrapPeerResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a PublishNetworkToBootstrapPeerResponse message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns PublishNetworkToBootstrapPeerResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): PublishNetworkToBootstrapPeerResponse;

    /**
     * Decodes a PublishNetworkToBootstrapPeerResponse message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns PublishNetworkToBootstrapPeerResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): PublishNetworkToBootstrapPeerResponse;

    /**
     * Verifies a PublishNetworkToBootstrapPeerResponse message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a PublishNetworkToBootstrapPeerResponse message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns PublishNetworkToBootstrapPeerResponse
     */
    public static fromObject(object: { [k: string]: any }): PublishNetworkToBootstrapPeerResponse;

    /**
     * Creates a plain object from a PublishNetworkToBootstrapPeerResponse message. Also converts values to other types if specified.
     * @param message PublishNetworkToBootstrapPeerResponse
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: PublishNetworkToBootstrapPeerResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this PublishNetworkToBootstrapPeerResponse to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a PeerExchangeMessage. */
export interface IPeerExchangeMessage {

    /** PeerExchangeMessage request */
    request?: (PeerExchangeMessage.IRequest|null);

    /** PeerExchangeMessage response */
    response?: (PeerExchangeMessage.IResponse|null);

    /** PeerExchangeMessage offer */
    offer?: (PeerExchangeMessage.IOffer|null);

    /** PeerExchangeMessage answer */
    answer?: (PeerExchangeMessage.IAnswer|null);

    /** PeerExchangeMessage iceCandidate */
    iceCandidate?: (PeerExchangeMessage.IIceCandidate|null);

    /** PeerExchangeMessage callbackRequest */
    callbackRequest?: (PeerExchangeMessage.ICallbackRequest|null);
}

/** Represents a PeerExchangeMessage. */
export class PeerExchangeMessage implements IPeerExchangeMessage {

    /**
     * Constructs a new PeerExchangeMessage.
     * @param [properties] Properties to set
     */
    constructor(properties?: IPeerExchangeMessage);

    /** PeerExchangeMessage request. */
    public request?: (PeerExchangeMessage.IRequest|null);

    /** PeerExchangeMessage response. */
    public response?: (PeerExchangeMessage.IResponse|null);

    /** PeerExchangeMessage offer. */
    public offer?: (PeerExchangeMessage.IOffer|null);

    /** PeerExchangeMessage answer. */
    public answer?: (PeerExchangeMessage.IAnswer|null);

    /** PeerExchangeMessage iceCandidate. */
    public iceCandidate?: (PeerExchangeMessage.IIceCandidate|null);

    /** PeerExchangeMessage callbackRequest. */
    public callbackRequest?: (PeerExchangeMessage.ICallbackRequest|null);

    /** PeerExchangeMessage body. */
    public body?: ("request"|"response"|"offer"|"answer"|"iceCandidate"|"callbackRequest");

    /**
     * Creates a new PeerExchangeMessage instance using the specified properties.
     * @param [properties] Properties to set
     * @returns PeerExchangeMessage instance
     */
    public static create(properties?: IPeerExchangeMessage): PeerExchangeMessage;

    /**
     * Encodes the specified PeerExchangeMessage message. Does not implicitly {@link PeerExchangeMessage.verify|verify} messages.
     * @param message PeerExchangeMessage message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IPeerExchangeMessage, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified PeerExchangeMessage message, length delimited. Does not implicitly {@link PeerExchangeMessage.verify|verify} messages.
     * @param message PeerExchangeMessage message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IPeerExchangeMessage, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a PeerExchangeMessage message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns PeerExchangeMessage
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): PeerExchangeMessage;

    /**
     * Decodes a PeerExchangeMessage message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns PeerExchangeMessage
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): PeerExchangeMessage;

    /**
     * Verifies a PeerExchangeMessage message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a PeerExchangeMessage message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns PeerExchangeMessage
     */
    public static fromObject(object: { [k: string]: any }): PeerExchangeMessage;

    /**
     * Creates a plain object from a PeerExchangeMessage message. Also converts values to other types if specified.
     * @param message PeerExchangeMessage
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: PeerExchangeMessage, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this PeerExchangeMessage to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

export namespace PeerExchangeMessage {

    /** Properties of a Request. */
    interface IRequest {

        /** Request count */
        count?: (number|null);
    }

    /** Represents a Request. */
    class Request implements IRequest {

        /**
         * Constructs a new Request.
         * @param [properties] Properties to set
         */
        constructor(properties?: PeerExchangeMessage.IRequest);

        /** Request count. */
        public count: number;

        /**
         * Creates a new Request instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Request instance
         */
        public static create(properties?: PeerExchangeMessage.IRequest): PeerExchangeMessage.Request;

        /**
         * Encodes the specified Request message. Does not implicitly {@link PeerExchangeMessage.Request.verify|verify} messages.
         * @param message Request message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: PeerExchangeMessage.IRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Request message, length delimited. Does not implicitly {@link PeerExchangeMessage.Request.verify|verify} messages.
         * @param message Request message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: PeerExchangeMessage.IRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a Request message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Request
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): PeerExchangeMessage.Request;

        /**
         * Decodes a Request message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Request
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): PeerExchangeMessage.Request;

        /**
         * Verifies a Request message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a Request message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Request
         */
        public static fromObject(object: { [k: string]: any }): PeerExchangeMessage.Request;

        /**
         * Creates a plain object from a Request message. Also converts values to other types if specified.
         * @param message Request
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: PeerExchangeMessage.Request, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Request to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }

    /** Properties of a Response. */
    interface IResponse {

        /** Response ids */
        ids?: (Uint8Array[]|null);
    }

    /** Represents a Response. */
    class Response implements IResponse {

        /**
         * Constructs a new Response.
         * @param [properties] Properties to set
         */
        constructor(properties?: PeerExchangeMessage.IResponse);

        /** Response ids. */
        public ids: Uint8Array[];

        /**
         * Creates a new Response instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Response instance
         */
        public static create(properties?: PeerExchangeMessage.IResponse): PeerExchangeMessage.Response;

        /**
         * Encodes the specified Response message. Does not implicitly {@link PeerExchangeMessage.Response.verify|verify} messages.
         * @param message Response message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: PeerExchangeMessage.IResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Response message, length delimited. Does not implicitly {@link PeerExchangeMessage.Response.verify|verify} messages.
         * @param message Response message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: PeerExchangeMessage.IResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a Response message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Response
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): PeerExchangeMessage.Response;

        /**
         * Decodes a Response message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Response
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): PeerExchangeMessage.Response;

        /**
         * Verifies a Response message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a Response message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Response
         */
        public static fromObject(object: { [k: string]: any }): PeerExchangeMessage.Response;

        /**
         * Creates a plain object from a Response message. Also converts values to other types if specified.
         * @param message Response
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: PeerExchangeMessage.Response, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Response to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }

    /** Properties of an Offer. */
    interface IOffer {

        /** Offer mediationId */
        mediationId?: (number|null);

        /** Offer data */
        data?: (Uint8Array|null);
    }

    /** Represents an Offer. */
    class Offer implements IOffer {

        /**
         * Constructs a new Offer.
         * @param [properties] Properties to set
         */
        constructor(properties?: PeerExchangeMessage.IOffer);

        /** Offer mediationId. */
        public mediationId: number;

        /** Offer data. */
        public data: Uint8Array;

        /**
         * Creates a new Offer instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Offer instance
         */
        public static create(properties?: PeerExchangeMessage.IOffer): PeerExchangeMessage.Offer;

        /**
         * Encodes the specified Offer message. Does not implicitly {@link PeerExchangeMessage.Offer.verify|verify} messages.
         * @param message Offer message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: PeerExchangeMessage.IOffer, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Offer message, length delimited. Does not implicitly {@link PeerExchangeMessage.Offer.verify|verify} messages.
         * @param message Offer message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: PeerExchangeMessage.IOffer, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes an Offer message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Offer
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): PeerExchangeMessage.Offer;

        /**
         * Decodes an Offer message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Offer
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): PeerExchangeMessage.Offer;

        /**
         * Verifies an Offer message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates an Offer message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Offer
         */
        public static fromObject(object: { [k: string]: any }): PeerExchangeMessage.Offer;

        /**
         * Creates a plain object from an Offer message. Also converts values to other types if specified.
         * @param message Offer
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: PeerExchangeMessage.Offer, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Offer to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }

    /** Properties of an Answer. */
    interface IAnswer {

        /** Answer mediationId */
        mediationId?: (number|null);

        /** Answer data */
        data?: (Uint8Array|null);
    }

    /** Represents an Answer. */
    class Answer implements IAnswer {

        /**
         * Constructs a new Answer.
         * @param [properties] Properties to set
         */
        constructor(properties?: PeerExchangeMessage.IAnswer);

        /** Answer mediationId. */
        public mediationId: number;

        /** Answer data. */
        public data: Uint8Array;

        /**
         * Creates a new Answer instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Answer instance
         */
        public static create(properties?: PeerExchangeMessage.IAnswer): PeerExchangeMessage.Answer;

        /**
         * Encodes the specified Answer message. Does not implicitly {@link PeerExchangeMessage.Answer.verify|verify} messages.
         * @param message Answer message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: PeerExchangeMessage.IAnswer, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Answer message, length delimited. Does not implicitly {@link PeerExchangeMessage.Answer.verify|verify} messages.
         * @param message Answer message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: PeerExchangeMessage.IAnswer, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes an Answer message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Answer
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): PeerExchangeMessage.Answer;

        /**
         * Decodes an Answer message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Answer
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): PeerExchangeMessage.Answer;

        /**
         * Verifies an Answer message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates an Answer message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Answer
         */
        public static fromObject(object: { [k: string]: any }): PeerExchangeMessage.Answer;

        /**
         * Creates a plain object from an Answer message. Also converts values to other types if specified.
         * @param message Answer
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: PeerExchangeMessage.Answer, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Answer to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }

    /** Properties of an IceCandidate. */
    interface IIceCandidate {

        /** IceCandidate mediationId */
        mediationId?: (number|null);

        /** IceCandidate index */
        index?: (number|null);

        /** IceCandidate data */
        data?: (Uint8Array|null);
    }

    /** Represents an IceCandidate. */
    class IceCandidate implements IIceCandidate {

        /**
         * Constructs a new IceCandidate.
         * @param [properties] Properties to set
         */
        constructor(properties?: PeerExchangeMessage.IIceCandidate);

        /** IceCandidate mediationId. */
        public mediationId: number;

        /** IceCandidate index. */
        public index: number;

        /** IceCandidate data. */
        public data: Uint8Array;

        /**
         * Creates a new IceCandidate instance using the specified properties.
         * @param [properties] Properties to set
         * @returns IceCandidate instance
         */
        public static create(properties?: PeerExchangeMessage.IIceCandidate): PeerExchangeMessage.IceCandidate;

        /**
         * Encodes the specified IceCandidate message. Does not implicitly {@link PeerExchangeMessage.IceCandidate.verify|verify} messages.
         * @param message IceCandidate message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: PeerExchangeMessage.IIceCandidate, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified IceCandidate message, length delimited. Does not implicitly {@link PeerExchangeMessage.IceCandidate.verify|verify} messages.
         * @param message IceCandidate message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: PeerExchangeMessage.IIceCandidate, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes an IceCandidate message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns IceCandidate
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): PeerExchangeMessage.IceCandidate;

        /**
         * Decodes an IceCandidate message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns IceCandidate
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): PeerExchangeMessage.IceCandidate;

        /**
         * Verifies an IceCandidate message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates an IceCandidate message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns IceCandidate
         */
        public static fromObject(object: { [k: string]: any }): PeerExchangeMessage.IceCandidate;

        /**
         * Creates a plain object from an IceCandidate message. Also converts values to other types if specified.
         * @param message IceCandidate
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: PeerExchangeMessage.IceCandidate, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this IceCandidate to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }

    /** Properties of a CallbackRequest. */
    interface ICallbackRequest {
    }

    /** Represents a CallbackRequest. */
    class CallbackRequest implements ICallbackRequest {

        /**
         * Constructs a new CallbackRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: PeerExchangeMessage.ICallbackRequest);

        /**
         * Creates a new CallbackRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns CallbackRequest instance
         */
        public static create(properties?: PeerExchangeMessage.ICallbackRequest): PeerExchangeMessage.CallbackRequest;

        /**
         * Encodes the specified CallbackRequest message. Does not implicitly {@link PeerExchangeMessage.CallbackRequest.verify|verify} messages.
         * @param message CallbackRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: PeerExchangeMessage.ICallbackRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified CallbackRequest message, length delimited. Does not implicitly {@link PeerExchangeMessage.CallbackRequest.verify|verify} messages.
         * @param message CallbackRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: PeerExchangeMessage.ICallbackRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a CallbackRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns CallbackRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): PeerExchangeMessage.CallbackRequest;

        /**
         * Decodes a CallbackRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns CallbackRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): PeerExchangeMessage.CallbackRequest;

        /**
         * Verifies a CallbackRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a CallbackRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns CallbackRequest
         */
        public static fromObject(object: { [k: string]: any }): PeerExchangeMessage.CallbackRequest;

        /**
         * Creates a plain object from a CallbackRequest message. Also converts values to other types if specified.
         * @param message CallbackRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: PeerExchangeMessage.CallbackRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this CallbackRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }
}

/** Properties of a CreateProfileRequest. */
export interface ICreateProfileRequest {

    /** CreateProfileRequest name */
    name?: (string|null);

    /** CreateProfileRequest password */
    password?: (string|null);
}

/** Represents a CreateProfileRequest. */
export class CreateProfileRequest implements ICreateProfileRequest {

    /**
     * Constructs a new CreateProfileRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: ICreateProfileRequest);

    /** CreateProfileRequest name. */
    public name: string;

    /** CreateProfileRequest password. */
    public password: string;

    /**
     * Creates a new CreateProfileRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns CreateProfileRequest instance
     */
    public static create(properties?: ICreateProfileRequest): CreateProfileRequest;

    /**
     * Encodes the specified CreateProfileRequest message. Does not implicitly {@link CreateProfileRequest.verify|verify} messages.
     * @param message CreateProfileRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: ICreateProfileRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified CreateProfileRequest message, length delimited. Does not implicitly {@link CreateProfileRequest.verify|verify} messages.
     * @param message CreateProfileRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: ICreateProfileRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a CreateProfileRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns CreateProfileRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): CreateProfileRequest;

    /**
     * Decodes a CreateProfileRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns CreateProfileRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): CreateProfileRequest;

    /**
     * Verifies a CreateProfileRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a CreateProfileRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns CreateProfileRequest
     */
    public static fromObject(object: { [k: string]: any }): CreateProfileRequest;

    /**
     * Creates a plain object from a CreateProfileRequest message. Also converts values to other types if specified.
     * @param message CreateProfileRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: CreateProfileRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this CreateProfileRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a CreateProfileResponse. */
export interface ICreateProfileResponse {

    /** CreateProfileResponse sessionId */
    sessionId?: (string|null);

    /** CreateProfileResponse profile */
    profile?: (IProfile|null);
}

/** Represents a CreateProfileResponse. */
export class CreateProfileResponse implements ICreateProfileResponse {

    /**
     * Constructs a new CreateProfileResponse.
     * @param [properties] Properties to set
     */
    constructor(properties?: ICreateProfileResponse);

    /** CreateProfileResponse sessionId. */
    public sessionId: string;

    /** CreateProfileResponse profile. */
    public profile?: (IProfile|null);

    /**
     * Creates a new CreateProfileResponse instance using the specified properties.
     * @param [properties] Properties to set
     * @returns CreateProfileResponse instance
     */
    public static create(properties?: ICreateProfileResponse): CreateProfileResponse;

    /**
     * Encodes the specified CreateProfileResponse message. Does not implicitly {@link CreateProfileResponse.verify|verify} messages.
     * @param message CreateProfileResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: ICreateProfileResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified CreateProfileResponse message, length delimited. Does not implicitly {@link CreateProfileResponse.verify|verify} messages.
     * @param message CreateProfileResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: ICreateProfileResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a CreateProfileResponse message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns CreateProfileResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): CreateProfileResponse;

    /**
     * Decodes a CreateProfileResponse message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns CreateProfileResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): CreateProfileResponse;

    /**
     * Verifies a CreateProfileResponse message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a CreateProfileResponse message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns CreateProfileResponse
     */
    public static fromObject(object: { [k: string]: any }): CreateProfileResponse;

    /**
     * Creates a plain object from a CreateProfileResponse message. Also converts values to other types if specified.
     * @param message CreateProfileResponse
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: CreateProfileResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this CreateProfileResponse to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of an UpdateProfileRequest. */
export interface IUpdateProfileRequest {

    /** UpdateProfileRequest name */
    name?: (string|null);

    /** UpdateProfileRequest password */
    password?: (string|null);
}

/** Represents an UpdateProfileRequest. */
export class UpdateProfileRequest implements IUpdateProfileRequest {

    /**
     * Constructs a new UpdateProfileRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: IUpdateProfileRequest);

    /** UpdateProfileRequest name. */
    public name: string;

    /** UpdateProfileRequest password. */
    public password: string;

    /**
     * Creates a new UpdateProfileRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns UpdateProfileRequest instance
     */
    public static create(properties?: IUpdateProfileRequest): UpdateProfileRequest;

    /**
     * Encodes the specified UpdateProfileRequest message. Does not implicitly {@link UpdateProfileRequest.verify|verify} messages.
     * @param message UpdateProfileRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IUpdateProfileRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified UpdateProfileRequest message, length delimited. Does not implicitly {@link UpdateProfileRequest.verify|verify} messages.
     * @param message UpdateProfileRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IUpdateProfileRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes an UpdateProfileRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns UpdateProfileRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): UpdateProfileRequest;

    /**
     * Decodes an UpdateProfileRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns UpdateProfileRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): UpdateProfileRequest;

    /**
     * Verifies an UpdateProfileRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates an UpdateProfileRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns UpdateProfileRequest
     */
    public static fromObject(object: { [k: string]: any }): UpdateProfileRequest;

    /**
     * Creates a plain object from an UpdateProfileRequest message. Also converts values to other types if specified.
     * @param message UpdateProfileRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: UpdateProfileRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this UpdateProfileRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of an UpdateProfileResponse. */
export interface IUpdateProfileResponse {

    /** UpdateProfileResponse profile */
    profile?: (IProfile|null);
}

/** Represents an UpdateProfileResponse. */
export class UpdateProfileResponse implements IUpdateProfileResponse {

    /**
     * Constructs a new UpdateProfileResponse.
     * @param [properties] Properties to set
     */
    constructor(properties?: IUpdateProfileResponse);

    /** UpdateProfileResponse profile. */
    public profile?: (IProfile|null);

    /**
     * Creates a new UpdateProfileResponse instance using the specified properties.
     * @param [properties] Properties to set
     * @returns UpdateProfileResponse instance
     */
    public static create(properties?: IUpdateProfileResponse): UpdateProfileResponse;

    /**
     * Encodes the specified UpdateProfileResponse message. Does not implicitly {@link UpdateProfileResponse.verify|verify} messages.
     * @param message UpdateProfileResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IUpdateProfileResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified UpdateProfileResponse message, length delimited. Does not implicitly {@link UpdateProfileResponse.verify|verify} messages.
     * @param message UpdateProfileResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IUpdateProfileResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes an UpdateProfileResponse message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns UpdateProfileResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): UpdateProfileResponse;

    /**
     * Decodes an UpdateProfileResponse message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns UpdateProfileResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): UpdateProfileResponse;

    /**
     * Verifies an UpdateProfileResponse message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates an UpdateProfileResponse message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns UpdateProfileResponse
     */
    public static fromObject(object: { [k: string]: any }): UpdateProfileResponse;

    /**
     * Creates a plain object from an UpdateProfileResponse message. Also converts values to other types if specified.
     * @param message UpdateProfileResponse
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: UpdateProfileResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this UpdateProfileResponse to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a DeleteProfileRequest. */
export interface IDeleteProfileRequest {

    /** DeleteProfileRequest id */
    id?: (number|null);
}

/** Represents a DeleteProfileRequest. */
export class DeleteProfileRequest implements IDeleteProfileRequest {

    /**
     * Constructs a new DeleteProfileRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: IDeleteProfileRequest);

    /** DeleteProfileRequest id. */
    public id: number;

    /**
     * Creates a new DeleteProfileRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns DeleteProfileRequest instance
     */
    public static create(properties?: IDeleteProfileRequest): DeleteProfileRequest;

    /**
     * Encodes the specified DeleteProfileRequest message. Does not implicitly {@link DeleteProfileRequest.verify|verify} messages.
     * @param message DeleteProfileRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IDeleteProfileRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified DeleteProfileRequest message, length delimited. Does not implicitly {@link DeleteProfileRequest.verify|verify} messages.
     * @param message DeleteProfileRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IDeleteProfileRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a DeleteProfileRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns DeleteProfileRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): DeleteProfileRequest;

    /**
     * Decodes a DeleteProfileRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns DeleteProfileRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): DeleteProfileRequest;

    /**
     * Verifies a DeleteProfileRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a DeleteProfileRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns DeleteProfileRequest
     */
    public static fromObject(object: { [k: string]: any }): DeleteProfileRequest;

    /**
     * Creates a plain object from a DeleteProfileRequest message. Also converts values to other types if specified.
     * @param message DeleteProfileRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: DeleteProfileRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this DeleteProfileRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a DeleteProfileResponse. */
export interface IDeleteProfileResponse {
}

/** Represents a DeleteProfileResponse. */
export class DeleteProfileResponse implements IDeleteProfileResponse {

    /**
     * Constructs a new DeleteProfileResponse.
     * @param [properties] Properties to set
     */
    constructor(properties?: IDeleteProfileResponse);

    /**
     * Creates a new DeleteProfileResponse instance using the specified properties.
     * @param [properties] Properties to set
     * @returns DeleteProfileResponse instance
     */
    public static create(properties?: IDeleteProfileResponse): DeleteProfileResponse;

    /**
     * Encodes the specified DeleteProfileResponse message. Does not implicitly {@link DeleteProfileResponse.verify|verify} messages.
     * @param message DeleteProfileResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IDeleteProfileResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified DeleteProfileResponse message, length delimited. Does not implicitly {@link DeleteProfileResponse.verify|verify} messages.
     * @param message DeleteProfileResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IDeleteProfileResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a DeleteProfileResponse message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns DeleteProfileResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): DeleteProfileResponse;

    /**
     * Decodes a DeleteProfileResponse message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns DeleteProfileResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): DeleteProfileResponse;

    /**
     * Verifies a DeleteProfileResponse message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a DeleteProfileResponse message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns DeleteProfileResponse
     */
    public static fromObject(object: { [k: string]: any }): DeleteProfileResponse;

    /**
     * Creates a plain object from a DeleteProfileResponse message. Also converts values to other types if specified.
     * @param message DeleteProfileResponse
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: DeleteProfileResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this DeleteProfileResponse to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a LoadProfileRequest. */
export interface ILoadProfileRequest {

    /** LoadProfileRequest id */
    id?: (number|null);

    /** LoadProfileRequest name */
    name?: (string|null);

    /** LoadProfileRequest password */
    password?: (string|null);
}

/** Represents a LoadProfileRequest. */
export class LoadProfileRequest implements ILoadProfileRequest {

    /**
     * Constructs a new LoadProfileRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: ILoadProfileRequest);

    /** LoadProfileRequest id. */
    public id: number;

    /** LoadProfileRequest name. */
    public name: string;

    /** LoadProfileRequest password. */
    public password: string;

    /**
     * Creates a new LoadProfileRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns LoadProfileRequest instance
     */
    public static create(properties?: ILoadProfileRequest): LoadProfileRequest;

    /**
     * Encodes the specified LoadProfileRequest message. Does not implicitly {@link LoadProfileRequest.verify|verify} messages.
     * @param message LoadProfileRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: ILoadProfileRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified LoadProfileRequest message, length delimited. Does not implicitly {@link LoadProfileRequest.verify|verify} messages.
     * @param message LoadProfileRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: ILoadProfileRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a LoadProfileRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns LoadProfileRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): LoadProfileRequest;

    /**
     * Decodes a LoadProfileRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns LoadProfileRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): LoadProfileRequest;

    /**
     * Verifies a LoadProfileRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a LoadProfileRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns LoadProfileRequest
     */
    public static fromObject(object: { [k: string]: any }): LoadProfileRequest;

    /**
     * Creates a plain object from a LoadProfileRequest message. Also converts values to other types if specified.
     * @param message LoadProfileRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: LoadProfileRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this LoadProfileRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a LoadProfileResponse. */
export interface ILoadProfileResponse {

    /** LoadProfileResponse sessionId */
    sessionId?: (string|null);

    /** LoadProfileResponse profile */
    profile?: (IProfile|null);
}

/** Represents a LoadProfileResponse. */
export class LoadProfileResponse implements ILoadProfileResponse {

    /**
     * Constructs a new LoadProfileResponse.
     * @param [properties] Properties to set
     */
    constructor(properties?: ILoadProfileResponse);

    /** LoadProfileResponse sessionId. */
    public sessionId: string;

    /** LoadProfileResponse profile. */
    public profile?: (IProfile|null);

    /**
     * Creates a new LoadProfileResponse instance using the specified properties.
     * @param [properties] Properties to set
     * @returns LoadProfileResponse instance
     */
    public static create(properties?: ILoadProfileResponse): LoadProfileResponse;

    /**
     * Encodes the specified LoadProfileResponse message. Does not implicitly {@link LoadProfileResponse.verify|verify} messages.
     * @param message LoadProfileResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: ILoadProfileResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified LoadProfileResponse message, length delimited. Does not implicitly {@link LoadProfileResponse.verify|verify} messages.
     * @param message LoadProfileResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: ILoadProfileResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a LoadProfileResponse message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns LoadProfileResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): LoadProfileResponse;

    /**
     * Decodes a LoadProfileResponse message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns LoadProfileResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): LoadProfileResponse;

    /**
     * Verifies a LoadProfileResponse message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a LoadProfileResponse message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns LoadProfileResponse
     */
    public static fromObject(object: { [k: string]: any }): LoadProfileResponse;

    /**
     * Creates a plain object from a LoadProfileResponse message. Also converts values to other types if specified.
     * @param message LoadProfileResponse
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: LoadProfileResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this LoadProfileResponse to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a GetProfileRequest. */
export interface IGetProfileRequest {

    /** GetProfileRequest sessionId */
    sessionId?: (string|null);
}

/** Represents a GetProfileRequest. */
export class GetProfileRequest implements IGetProfileRequest {

    /**
     * Constructs a new GetProfileRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: IGetProfileRequest);

    /** GetProfileRequest sessionId. */
    public sessionId: string;

    /**
     * Creates a new GetProfileRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns GetProfileRequest instance
     */
    public static create(properties?: IGetProfileRequest): GetProfileRequest;

    /**
     * Encodes the specified GetProfileRequest message. Does not implicitly {@link GetProfileRequest.verify|verify} messages.
     * @param message GetProfileRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IGetProfileRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified GetProfileRequest message, length delimited. Does not implicitly {@link GetProfileRequest.verify|verify} messages.
     * @param message GetProfileRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IGetProfileRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a GetProfileRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns GetProfileRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): GetProfileRequest;

    /**
     * Decodes a GetProfileRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns GetProfileRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): GetProfileRequest;

    /**
     * Verifies a GetProfileRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a GetProfileRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns GetProfileRequest
     */
    public static fromObject(object: { [k: string]: any }): GetProfileRequest;

    /**
     * Creates a plain object from a GetProfileRequest message. Also converts values to other types if specified.
     * @param message GetProfileRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: GetProfileRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this GetProfileRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a GetProfileResponse. */
export interface IGetProfileResponse {

    /** GetProfileResponse profile */
    profile?: (IProfile|null);
}

/** Represents a GetProfileResponse. */
export class GetProfileResponse implements IGetProfileResponse {

    /**
     * Constructs a new GetProfileResponse.
     * @param [properties] Properties to set
     */
    constructor(properties?: IGetProfileResponse);

    /** GetProfileResponse profile. */
    public profile?: (IProfile|null);

    /**
     * Creates a new GetProfileResponse instance using the specified properties.
     * @param [properties] Properties to set
     * @returns GetProfileResponse instance
     */
    public static create(properties?: IGetProfileResponse): GetProfileResponse;

    /**
     * Encodes the specified GetProfileResponse message. Does not implicitly {@link GetProfileResponse.verify|verify} messages.
     * @param message GetProfileResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IGetProfileResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified GetProfileResponse message, length delimited. Does not implicitly {@link GetProfileResponse.verify|verify} messages.
     * @param message GetProfileResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IGetProfileResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a GetProfileResponse message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns GetProfileResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): GetProfileResponse;

    /**
     * Decodes a GetProfileResponse message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns GetProfileResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): GetProfileResponse;

    /**
     * Verifies a GetProfileResponse message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a GetProfileResponse message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns GetProfileResponse
     */
    public static fromObject(object: { [k: string]: any }): GetProfileResponse;

    /**
     * Creates a plain object from a GetProfileResponse message. Also converts values to other types if specified.
     * @param message GetProfileResponse
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: GetProfileResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this GetProfileResponse to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a GetProfilesRequest. */
export interface IGetProfilesRequest {
}

/** Represents a GetProfilesRequest. */
export class GetProfilesRequest implements IGetProfilesRequest {

    /**
     * Constructs a new GetProfilesRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: IGetProfilesRequest);

    /**
     * Creates a new GetProfilesRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns GetProfilesRequest instance
     */
    public static create(properties?: IGetProfilesRequest): GetProfilesRequest;

    /**
     * Encodes the specified GetProfilesRequest message. Does not implicitly {@link GetProfilesRequest.verify|verify} messages.
     * @param message GetProfilesRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IGetProfilesRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified GetProfilesRequest message, length delimited. Does not implicitly {@link GetProfilesRequest.verify|verify} messages.
     * @param message GetProfilesRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IGetProfilesRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a GetProfilesRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns GetProfilesRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): GetProfilesRequest;

    /**
     * Decodes a GetProfilesRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns GetProfilesRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): GetProfilesRequest;

    /**
     * Verifies a GetProfilesRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a GetProfilesRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns GetProfilesRequest
     */
    public static fromObject(object: { [k: string]: any }): GetProfilesRequest;

    /**
     * Creates a plain object from a GetProfilesRequest message. Also converts values to other types if specified.
     * @param message GetProfilesRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: GetProfilesRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this GetProfilesRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a GetProfilesResponse. */
export interface IGetProfilesResponse {

    /** GetProfilesResponse profiles */
    profiles?: (IProfileSummary[]|null);
}

/** Represents a GetProfilesResponse. */
export class GetProfilesResponse implements IGetProfilesResponse {

    /**
     * Constructs a new GetProfilesResponse.
     * @param [properties] Properties to set
     */
    constructor(properties?: IGetProfilesResponse);

    /** GetProfilesResponse profiles. */
    public profiles: IProfileSummary[];

    /**
     * Creates a new GetProfilesResponse instance using the specified properties.
     * @param [properties] Properties to set
     * @returns GetProfilesResponse instance
     */
    public static create(properties?: IGetProfilesResponse): GetProfilesResponse;

    /**
     * Encodes the specified GetProfilesResponse message. Does not implicitly {@link GetProfilesResponse.verify|verify} messages.
     * @param message GetProfilesResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IGetProfilesResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified GetProfilesResponse message, length delimited. Does not implicitly {@link GetProfilesResponse.verify|verify} messages.
     * @param message GetProfilesResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IGetProfilesResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a GetProfilesResponse message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns GetProfilesResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): GetProfilesResponse;

    /**
     * Decodes a GetProfilesResponse message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns GetProfilesResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): GetProfilesResponse;

    /**
     * Verifies a GetProfilesResponse message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a GetProfilesResponse message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns GetProfilesResponse
     */
    public static fromObject(object: { [k: string]: any }): GetProfilesResponse;

    /**
     * Creates a plain object from a GetProfilesResponse message. Also converts values to other types if specified.
     * @param message GetProfilesResponse
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: GetProfilesResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this GetProfilesResponse to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a LoadSessionRequest. */
export interface ILoadSessionRequest {

    /** LoadSessionRequest sessionId */
    sessionId?: (string|null);
}

/** Represents a LoadSessionRequest. */
export class LoadSessionRequest implements ILoadSessionRequest {

    /**
     * Constructs a new LoadSessionRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: ILoadSessionRequest);

    /** LoadSessionRequest sessionId. */
    public sessionId: string;

    /**
     * Creates a new LoadSessionRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns LoadSessionRequest instance
     */
    public static create(properties?: ILoadSessionRequest): LoadSessionRequest;

    /**
     * Encodes the specified LoadSessionRequest message. Does not implicitly {@link LoadSessionRequest.verify|verify} messages.
     * @param message LoadSessionRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: ILoadSessionRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified LoadSessionRequest message, length delimited. Does not implicitly {@link LoadSessionRequest.verify|verify} messages.
     * @param message LoadSessionRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: ILoadSessionRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a LoadSessionRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns LoadSessionRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): LoadSessionRequest;

    /**
     * Decodes a LoadSessionRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns LoadSessionRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): LoadSessionRequest;

    /**
     * Verifies a LoadSessionRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a LoadSessionRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns LoadSessionRequest
     */
    public static fromObject(object: { [k: string]: any }): LoadSessionRequest;

    /**
     * Creates a plain object from a LoadSessionRequest message. Also converts values to other types if specified.
     * @param message LoadSessionRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: LoadSessionRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this LoadSessionRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a LoadSessionResponse. */
export interface ILoadSessionResponse {

    /** LoadSessionResponse sessionId */
    sessionId?: (string|null);

    /** LoadSessionResponse profile */
    profile?: (IProfile|null);
}

/** Represents a LoadSessionResponse. */
export class LoadSessionResponse implements ILoadSessionResponse {

    /**
     * Constructs a new LoadSessionResponse.
     * @param [properties] Properties to set
     */
    constructor(properties?: ILoadSessionResponse);

    /** LoadSessionResponse sessionId. */
    public sessionId: string;

    /** LoadSessionResponse profile. */
    public profile?: (IProfile|null);

    /**
     * Creates a new LoadSessionResponse instance using the specified properties.
     * @param [properties] Properties to set
     * @returns LoadSessionResponse instance
     */
    public static create(properties?: ILoadSessionResponse): LoadSessionResponse;

    /**
     * Encodes the specified LoadSessionResponse message. Does not implicitly {@link LoadSessionResponse.verify|verify} messages.
     * @param message LoadSessionResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: ILoadSessionResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified LoadSessionResponse message, length delimited. Does not implicitly {@link LoadSessionResponse.verify|verify} messages.
     * @param message LoadSessionResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: ILoadSessionResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a LoadSessionResponse message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns LoadSessionResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): LoadSessionResponse;

    /**
     * Decodes a LoadSessionResponse message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns LoadSessionResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): LoadSessionResponse;

    /**
     * Verifies a LoadSessionResponse message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a LoadSessionResponse message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns LoadSessionResponse
     */
    public static fromObject(object: { [k: string]: any }): LoadSessionResponse;

    /**
     * Creates a plain object from a LoadSessionResponse message. Also converts values to other types if specified.
     * @param message LoadSessionResponse
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: LoadSessionResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this LoadSessionResponse to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a NetworkIcon. */
export interface INetworkIcon {

    /** NetworkIcon data */
    data?: (Uint8Array|null);

    /** NetworkIcon type */
    type?: (string|null);
}

/** Represents a NetworkIcon. */
export class NetworkIcon implements INetworkIcon {

    /**
     * Constructs a new NetworkIcon.
     * @param [properties] Properties to set
     */
    constructor(properties?: INetworkIcon);

    /** NetworkIcon data. */
    public data: Uint8Array;

    /** NetworkIcon type. */
    public type: string;

    /**
     * Creates a new NetworkIcon instance using the specified properties.
     * @param [properties] Properties to set
     * @returns NetworkIcon instance
     */
    public static create(properties?: INetworkIcon): NetworkIcon;

    /**
     * Encodes the specified NetworkIcon message. Does not implicitly {@link NetworkIcon.verify|verify} messages.
     * @param message NetworkIcon message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: INetworkIcon, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified NetworkIcon message, length delimited. Does not implicitly {@link NetworkIcon.verify|verify} messages.
     * @param message NetworkIcon message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: INetworkIcon, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a NetworkIcon message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns NetworkIcon
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): NetworkIcon;

    /**
     * Decodes a NetworkIcon message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns NetworkIcon
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): NetworkIcon;

    /**
     * Verifies a NetworkIcon message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a NetworkIcon message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns NetworkIcon
     */
    public static fromObject(object: { [k: string]: any }): NetworkIcon;

    /**
     * Creates a plain object from a NetworkIcon message. Also converts values to other types if specified.
     * @param message NetworkIcon
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: NetworkIcon, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this NetworkIcon to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a CreateNetworkRequest. */
export interface ICreateNetworkRequest {

    /** CreateNetworkRequest name */
    name?: (string|null);

    /** CreateNetworkRequest icon */
    icon?: (INetworkIcon|null);
}

/** Represents a CreateNetworkRequest. */
export class CreateNetworkRequest implements ICreateNetworkRequest {

    /**
     * Constructs a new CreateNetworkRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: ICreateNetworkRequest);

    /** CreateNetworkRequest name. */
    public name: string;

    /** CreateNetworkRequest icon. */
    public icon?: (INetworkIcon|null);

    /**
     * Creates a new CreateNetworkRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns CreateNetworkRequest instance
     */
    public static create(properties?: ICreateNetworkRequest): CreateNetworkRequest;

    /**
     * Encodes the specified CreateNetworkRequest message. Does not implicitly {@link CreateNetworkRequest.verify|verify} messages.
     * @param message CreateNetworkRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: ICreateNetworkRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified CreateNetworkRequest message, length delimited. Does not implicitly {@link CreateNetworkRequest.verify|verify} messages.
     * @param message CreateNetworkRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: ICreateNetworkRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a CreateNetworkRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns CreateNetworkRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): CreateNetworkRequest;

    /**
     * Decodes a CreateNetworkRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns CreateNetworkRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): CreateNetworkRequest;

    /**
     * Verifies a CreateNetworkRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a CreateNetworkRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns CreateNetworkRequest
     */
    public static fromObject(object: { [k: string]: any }): CreateNetworkRequest;

    /**
     * Creates a plain object from a CreateNetworkRequest message. Also converts values to other types if specified.
     * @param message CreateNetworkRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: CreateNetworkRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this CreateNetworkRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a CreateNetworkResponse. */
export interface ICreateNetworkResponse {

    /** CreateNetworkResponse network */
    network?: (INetwork|null);
}

/** Represents a CreateNetworkResponse. */
export class CreateNetworkResponse implements ICreateNetworkResponse {

    /**
     * Constructs a new CreateNetworkResponse.
     * @param [properties] Properties to set
     */
    constructor(properties?: ICreateNetworkResponse);

    /** CreateNetworkResponse network. */
    public network?: (INetwork|null);

    /**
     * Creates a new CreateNetworkResponse instance using the specified properties.
     * @param [properties] Properties to set
     * @returns CreateNetworkResponse instance
     */
    public static create(properties?: ICreateNetworkResponse): CreateNetworkResponse;

    /**
     * Encodes the specified CreateNetworkResponse message. Does not implicitly {@link CreateNetworkResponse.verify|verify} messages.
     * @param message CreateNetworkResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: ICreateNetworkResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified CreateNetworkResponse message, length delimited. Does not implicitly {@link CreateNetworkResponse.verify|verify} messages.
     * @param message CreateNetworkResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: ICreateNetworkResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a CreateNetworkResponse message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns CreateNetworkResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): CreateNetworkResponse;

    /**
     * Decodes a CreateNetworkResponse message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns CreateNetworkResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): CreateNetworkResponse;

    /**
     * Verifies a CreateNetworkResponse message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a CreateNetworkResponse message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns CreateNetworkResponse
     */
    public static fromObject(object: { [k: string]: any }): CreateNetworkResponse;

    /**
     * Creates a plain object from a CreateNetworkResponse message. Also converts values to other types if specified.
     * @param message CreateNetworkResponse
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: CreateNetworkResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this CreateNetworkResponse to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of an UpdateNetworkRequest. */
export interface IUpdateNetworkRequest {

    /** UpdateNetworkRequest id */
    id?: (number|null);

    /** UpdateNetworkRequest name */
    name?: (string|null);
}

/** Represents an UpdateNetworkRequest. */
export class UpdateNetworkRequest implements IUpdateNetworkRequest {

    /**
     * Constructs a new UpdateNetworkRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: IUpdateNetworkRequest);

    /** UpdateNetworkRequest id. */
    public id: number;

    /** UpdateNetworkRequest name. */
    public name: string;

    /**
     * Creates a new UpdateNetworkRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns UpdateNetworkRequest instance
     */
    public static create(properties?: IUpdateNetworkRequest): UpdateNetworkRequest;

    /**
     * Encodes the specified UpdateNetworkRequest message. Does not implicitly {@link UpdateNetworkRequest.verify|verify} messages.
     * @param message UpdateNetworkRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IUpdateNetworkRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified UpdateNetworkRequest message, length delimited. Does not implicitly {@link UpdateNetworkRequest.verify|verify} messages.
     * @param message UpdateNetworkRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IUpdateNetworkRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes an UpdateNetworkRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns UpdateNetworkRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): UpdateNetworkRequest;

    /**
     * Decodes an UpdateNetworkRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns UpdateNetworkRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): UpdateNetworkRequest;

    /**
     * Verifies an UpdateNetworkRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates an UpdateNetworkRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns UpdateNetworkRequest
     */
    public static fromObject(object: { [k: string]: any }): UpdateNetworkRequest;

    /**
     * Creates a plain object from an UpdateNetworkRequest message. Also converts values to other types if specified.
     * @param message UpdateNetworkRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: UpdateNetworkRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this UpdateNetworkRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of an UpdateNetworkResponse. */
export interface IUpdateNetworkResponse {

    /** UpdateNetworkResponse network */
    network?: (INetwork|null);
}

/** Represents an UpdateNetworkResponse. */
export class UpdateNetworkResponse implements IUpdateNetworkResponse {

    /**
     * Constructs a new UpdateNetworkResponse.
     * @param [properties] Properties to set
     */
    constructor(properties?: IUpdateNetworkResponse);

    /** UpdateNetworkResponse network. */
    public network?: (INetwork|null);

    /**
     * Creates a new UpdateNetworkResponse instance using the specified properties.
     * @param [properties] Properties to set
     * @returns UpdateNetworkResponse instance
     */
    public static create(properties?: IUpdateNetworkResponse): UpdateNetworkResponse;

    /**
     * Encodes the specified UpdateNetworkResponse message. Does not implicitly {@link UpdateNetworkResponse.verify|verify} messages.
     * @param message UpdateNetworkResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IUpdateNetworkResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified UpdateNetworkResponse message, length delimited. Does not implicitly {@link UpdateNetworkResponse.verify|verify} messages.
     * @param message UpdateNetworkResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IUpdateNetworkResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes an UpdateNetworkResponse message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns UpdateNetworkResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): UpdateNetworkResponse;

    /**
     * Decodes an UpdateNetworkResponse message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns UpdateNetworkResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): UpdateNetworkResponse;

    /**
     * Verifies an UpdateNetworkResponse message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates an UpdateNetworkResponse message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns UpdateNetworkResponse
     */
    public static fromObject(object: { [k: string]: any }): UpdateNetworkResponse;

    /**
     * Creates a plain object from an UpdateNetworkResponse message. Also converts values to other types if specified.
     * @param message UpdateNetworkResponse
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: UpdateNetworkResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this UpdateNetworkResponse to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a DeleteNetworkRequest. */
export interface IDeleteNetworkRequest {

    /** DeleteNetworkRequest id */
    id?: (number|null);
}

/** Represents a DeleteNetworkRequest. */
export class DeleteNetworkRequest implements IDeleteNetworkRequest {

    /**
     * Constructs a new DeleteNetworkRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: IDeleteNetworkRequest);

    /** DeleteNetworkRequest id. */
    public id: number;

    /**
     * Creates a new DeleteNetworkRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns DeleteNetworkRequest instance
     */
    public static create(properties?: IDeleteNetworkRequest): DeleteNetworkRequest;

    /**
     * Encodes the specified DeleteNetworkRequest message. Does not implicitly {@link DeleteNetworkRequest.verify|verify} messages.
     * @param message DeleteNetworkRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IDeleteNetworkRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified DeleteNetworkRequest message, length delimited. Does not implicitly {@link DeleteNetworkRequest.verify|verify} messages.
     * @param message DeleteNetworkRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IDeleteNetworkRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a DeleteNetworkRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns DeleteNetworkRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): DeleteNetworkRequest;

    /**
     * Decodes a DeleteNetworkRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns DeleteNetworkRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): DeleteNetworkRequest;

    /**
     * Verifies a DeleteNetworkRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a DeleteNetworkRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns DeleteNetworkRequest
     */
    public static fromObject(object: { [k: string]: any }): DeleteNetworkRequest;

    /**
     * Creates a plain object from a DeleteNetworkRequest message. Also converts values to other types if specified.
     * @param message DeleteNetworkRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: DeleteNetworkRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this DeleteNetworkRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a DeleteNetworkResponse. */
export interface IDeleteNetworkResponse {
}

/** Represents a DeleteNetworkResponse. */
export class DeleteNetworkResponse implements IDeleteNetworkResponse {

    /**
     * Constructs a new DeleteNetworkResponse.
     * @param [properties] Properties to set
     */
    constructor(properties?: IDeleteNetworkResponse);

    /**
     * Creates a new DeleteNetworkResponse instance using the specified properties.
     * @param [properties] Properties to set
     * @returns DeleteNetworkResponse instance
     */
    public static create(properties?: IDeleteNetworkResponse): DeleteNetworkResponse;

    /**
     * Encodes the specified DeleteNetworkResponse message. Does not implicitly {@link DeleteNetworkResponse.verify|verify} messages.
     * @param message DeleteNetworkResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IDeleteNetworkResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified DeleteNetworkResponse message, length delimited. Does not implicitly {@link DeleteNetworkResponse.verify|verify} messages.
     * @param message DeleteNetworkResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IDeleteNetworkResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a DeleteNetworkResponse message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns DeleteNetworkResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): DeleteNetworkResponse;

    /**
     * Decodes a DeleteNetworkResponse message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns DeleteNetworkResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): DeleteNetworkResponse;

    /**
     * Verifies a DeleteNetworkResponse message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a DeleteNetworkResponse message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns DeleteNetworkResponse
     */
    public static fromObject(object: { [k: string]: any }): DeleteNetworkResponse;

    /**
     * Creates a plain object from a DeleteNetworkResponse message. Also converts values to other types if specified.
     * @param message DeleteNetworkResponse
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: DeleteNetworkResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this DeleteNetworkResponse to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a GetNetworkRequest. */
export interface IGetNetworkRequest {

    /** GetNetworkRequest id */
    id?: (number|null);
}

/** Represents a GetNetworkRequest. */
export class GetNetworkRequest implements IGetNetworkRequest {

    /**
     * Constructs a new GetNetworkRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: IGetNetworkRequest);

    /** GetNetworkRequest id. */
    public id: number;

    /**
     * Creates a new GetNetworkRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns GetNetworkRequest instance
     */
    public static create(properties?: IGetNetworkRequest): GetNetworkRequest;

    /**
     * Encodes the specified GetNetworkRequest message. Does not implicitly {@link GetNetworkRequest.verify|verify} messages.
     * @param message GetNetworkRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IGetNetworkRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified GetNetworkRequest message, length delimited. Does not implicitly {@link GetNetworkRequest.verify|verify} messages.
     * @param message GetNetworkRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IGetNetworkRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a GetNetworkRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns GetNetworkRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): GetNetworkRequest;

    /**
     * Decodes a GetNetworkRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns GetNetworkRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): GetNetworkRequest;

    /**
     * Verifies a GetNetworkRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a GetNetworkRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns GetNetworkRequest
     */
    public static fromObject(object: { [k: string]: any }): GetNetworkRequest;

    /**
     * Creates a plain object from a GetNetworkRequest message. Also converts values to other types if specified.
     * @param message GetNetworkRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: GetNetworkRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this GetNetworkRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a GetNetworkResponse. */
export interface IGetNetworkResponse {

    /** GetNetworkResponse network */
    network?: (INetwork|null);
}

/** Represents a GetNetworkResponse. */
export class GetNetworkResponse implements IGetNetworkResponse {

    /**
     * Constructs a new GetNetworkResponse.
     * @param [properties] Properties to set
     */
    constructor(properties?: IGetNetworkResponse);

    /** GetNetworkResponse network. */
    public network?: (INetwork|null);

    /**
     * Creates a new GetNetworkResponse instance using the specified properties.
     * @param [properties] Properties to set
     * @returns GetNetworkResponse instance
     */
    public static create(properties?: IGetNetworkResponse): GetNetworkResponse;

    /**
     * Encodes the specified GetNetworkResponse message. Does not implicitly {@link GetNetworkResponse.verify|verify} messages.
     * @param message GetNetworkResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IGetNetworkResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified GetNetworkResponse message, length delimited. Does not implicitly {@link GetNetworkResponse.verify|verify} messages.
     * @param message GetNetworkResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IGetNetworkResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a GetNetworkResponse message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns GetNetworkResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): GetNetworkResponse;

    /**
     * Decodes a GetNetworkResponse message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns GetNetworkResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): GetNetworkResponse;

    /**
     * Verifies a GetNetworkResponse message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a GetNetworkResponse message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns GetNetworkResponse
     */
    public static fromObject(object: { [k: string]: any }): GetNetworkResponse;

    /**
     * Creates a plain object from a GetNetworkResponse message. Also converts values to other types if specified.
     * @param message GetNetworkResponse
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: GetNetworkResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this GetNetworkResponse to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a GetNetworksRequest. */
export interface IGetNetworksRequest {
}

/** Represents a GetNetworksRequest. */
export class GetNetworksRequest implements IGetNetworksRequest {

    /**
     * Constructs a new GetNetworksRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: IGetNetworksRequest);

    /**
     * Creates a new GetNetworksRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns GetNetworksRequest instance
     */
    public static create(properties?: IGetNetworksRequest): GetNetworksRequest;

    /**
     * Encodes the specified GetNetworksRequest message. Does not implicitly {@link GetNetworksRequest.verify|verify} messages.
     * @param message GetNetworksRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IGetNetworksRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified GetNetworksRequest message, length delimited. Does not implicitly {@link GetNetworksRequest.verify|verify} messages.
     * @param message GetNetworksRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IGetNetworksRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a GetNetworksRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns GetNetworksRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): GetNetworksRequest;

    /**
     * Decodes a GetNetworksRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns GetNetworksRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): GetNetworksRequest;

    /**
     * Verifies a GetNetworksRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a GetNetworksRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns GetNetworksRequest
     */
    public static fromObject(object: { [k: string]: any }): GetNetworksRequest;

    /**
     * Creates a plain object from a GetNetworksRequest message. Also converts values to other types if specified.
     * @param message GetNetworksRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: GetNetworksRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this GetNetworksRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a GetNetworksResponse. */
export interface IGetNetworksResponse {

    /** GetNetworksResponse networks */
    networks?: (INetwork[]|null);
}

/** Represents a GetNetworksResponse. */
export class GetNetworksResponse implements IGetNetworksResponse {

    /**
     * Constructs a new GetNetworksResponse.
     * @param [properties] Properties to set
     */
    constructor(properties?: IGetNetworksResponse);

    /** GetNetworksResponse networks. */
    public networks: INetwork[];

    /**
     * Creates a new GetNetworksResponse instance using the specified properties.
     * @param [properties] Properties to set
     * @returns GetNetworksResponse instance
     */
    public static create(properties?: IGetNetworksResponse): GetNetworksResponse;

    /**
     * Encodes the specified GetNetworksResponse message. Does not implicitly {@link GetNetworksResponse.verify|verify} messages.
     * @param message GetNetworksResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IGetNetworksResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified GetNetworksResponse message, length delimited. Does not implicitly {@link GetNetworksResponse.verify|verify} messages.
     * @param message GetNetworksResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IGetNetworksResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a GetNetworksResponse message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns GetNetworksResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): GetNetworksResponse;

    /**
     * Decodes a GetNetworksResponse message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns GetNetworksResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): GetNetworksResponse;

    /**
     * Verifies a GetNetworksResponse message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a GetNetworksResponse message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns GetNetworksResponse
     */
    public static fromObject(object: { [k: string]: any }): GetNetworksResponse;

    /**
     * Creates a plain object from a GetNetworksResponse message. Also converts values to other types if specified.
     * @param message GetNetworksResponse
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: GetNetworksResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this GetNetworksResponse to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a GetNetworkMembershipsRequest. */
export interface IGetNetworkMembershipsRequest {
}

/** Represents a GetNetworkMembershipsRequest. */
export class GetNetworkMembershipsRequest implements IGetNetworkMembershipsRequest {

    /**
     * Constructs a new GetNetworkMembershipsRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: IGetNetworkMembershipsRequest);

    /**
     * Creates a new GetNetworkMembershipsRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns GetNetworkMembershipsRequest instance
     */
    public static create(properties?: IGetNetworkMembershipsRequest): GetNetworkMembershipsRequest;

    /**
     * Encodes the specified GetNetworkMembershipsRequest message. Does not implicitly {@link GetNetworkMembershipsRequest.verify|verify} messages.
     * @param message GetNetworkMembershipsRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IGetNetworkMembershipsRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified GetNetworkMembershipsRequest message, length delimited. Does not implicitly {@link GetNetworkMembershipsRequest.verify|verify} messages.
     * @param message GetNetworkMembershipsRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IGetNetworkMembershipsRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a GetNetworkMembershipsRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns GetNetworkMembershipsRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): GetNetworkMembershipsRequest;

    /**
     * Decodes a GetNetworkMembershipsRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns GetNetworkMembershipsRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): GetNetworkMembershipsRequest;

    /**
     * Verifies a GetNetworkMembershipsRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a GetNetworkMembershipsRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns GetNetworkMembershipsRequest
     */
    public static fromObject(object: { [k: string]: any }): GetNetworkMembershipsRequest;

    /**
     * Creates a plain object from a GetNetworkMembershipsRequest message. Also converts values to other types if specified.
     * @param message GetNetworkMembershipsRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: GetNetworkMembershipsRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this GetNetworkMembershipsRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a GetNetworkMembershipsResponse. */
export interface IGetNetworkMembershipsResponse {

    /** GetNetworkMembershipsResponse networkMemberships */
    networkMemberships?: (INetworkMembership[]|null);
}

/** Represents a GetNetworkMembershipsResponse. */
export class GetNetworkMembershipsResponse implements IGetNetworkMembershipsResponse {

    /**
     * Constructs a new GetNetworkMembershipsResponse.
     * @param [properties] Properties to set
     */
    constructor(properties?: IGetNetworkMembershipsResponse);

    /** GetNetworkMembershipsResponse networkMemberships. */
    public networkMemberships: INetworkMembership[];

    /**
     * Creates a new GetNetworkMembershipsResponse instance using the specified properties.
     * @param [properties] Properties to set
     * @returns GetNetworkMembershipsResponse instance
     */
    public static create(properties?: IGetNetworkMembershipsResponse): GetNetworkMembershipsResponse;

    /**
     * Encodes the specified GetNetworkMembershipsResponse message. Does not implicitly {@link GetNetworkMembershipsResponse.verify|verify} messages.
     * @param message GetNetworkMembershipsResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IGetNetworkMembershipsResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified GetNetworkMembershipsResponse message, length delimited. Does not implicitly {@link GetNetworkMembershipsResponse.verify|verify} messages.
     * @param message GetNetworkMembershipsResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IGetNetworkMembershipsResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a GetNetworkMembershipsResponse message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns GetNetworkMembershipsResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): GetNetworkMembershipsResponse;

    /**
     * Decodes a GetNetworkMembershipsResponse message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns GetNetworkMembershipsResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): GetNetworkMembershipsResponse;

    /**
     * Verifies a GetNetworkMembershipsResponse message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a GetNetworkMembershipsResponse message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns GetNetworkMembershipsResponse
     */
    public static fromObject(object: { [k: string]: any }): GetNetworkMembershipsResponse;

    /**
     * Creates a plain object from a GetNetworkMembershipsResponse message. Also converts values to other types if specified.
     * @param message GetNetworkMembershipsResponse
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: GetNetworkMembershipsResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this GetNetworkMembershipsResponse to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a DeleteNetworkMembershipRequest. */
export interface IDeleteNetworkMembershipRequest {

    /** DeleteNetworkMembershipRequest id */
    id?: (number|null);
}

/** Represents a DeleteNetworkMembershipRequest. */
export class DeleteNetworkMembershipRequest implements IDeleteNetworkMembershipRequest {

    /**
     * Constructs a new DeleteNetworkMembershipRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: IDeleteNetworkMembershipRequest);

    /** DeleteNetworkMembershipRequest id. */
    public id: number;

    /**
     * Creates a new DeleteNetworkMembershipRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns DeleteNetworkMembershipRequest instance
     */
    public static create(properties?: IDeleteNetworkMembershipRequest): DeleteNetworkMembershipRequest;

    /**
     * Encodes the specified DeleteNetworkMembershipRequest message. Does not implicitly {@link DeleteNetworkMembershipRequest.verify|verify} messages.
     * @param message DeleteNetworkMembershipRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IDeleteNetworkMembershipRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified DeleteNetworkMembershipRequest message, length delimited. Does not implicitly {@link DeleteNetworkMembershipRequest.verify|verify} messages.
     * @param message DeleteNetworkMembershipRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IDeleteNetworkMembershipRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a DeleteNetworkMembershipRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns DeleteNetworkMembershipRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): DeleteNetworkMembershipRequest;

    /**
     * Decodes a DeleteNetworkMembershipRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns DeleteNetworkMembershipRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): DeleteNetworkMembershipRequest;

    /**
     * Verifies a DeleteNetworkMembershipRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a DeleteNetworkMembershipRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns DeleteNetworkMembershipRequest
     */
    public static fromObject(object: { [k: string]: any }): DeleteNetworkMembershipRequest;

    /**
     * Creates a plain object from a DeleteNetworkMembershipRequest message. Also converts values to other types if specified.
     * @param message DeleteNetworkMembershipRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: DeleteNetworkMembershipRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this DeleteNetworkMembershipRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a DeleteNetworkMembershipResponse. */
export interface IDeleteNetworkMembershipResponse {
}

/** Represents a DeleteNetworkMembershipResponse. */
export class DeleteNetworkMembershipResponse implements IDeleteNetworkMembershipResponse {

    /**
     * Constructs a new DeleteNetworkMembershipResponse.
     * @param [properties] Properties to set
     */
    constructor(properties?: IDeleteNetworkMembershipResponse);

    /**
     * Creates a new DeleteNetworkMembershipResponse instance using the specified properties.
     * @param [properties] Properties to set
     * @returns DeleteNetworkMembershipResponse instance
     */
    public static create(properties?: IDeleteNetworkMembershipResponse): DeleteNetworkMembershipResponse;

    /**
     * Encodes the specified DeleteNetworkMembershipResponse message. Does not implicitly {@link DeleteNetworkMembershipResponse.verify|verify} messages.
     * @param message DeleteNetworkMembershipResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IDeleteNetworkMembershipResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified DeleteNetworkMembershipResponse message, length delimited. Does not implicitly {@link DeleteNetworkMembershipResponse.verify|verify} messages.
     * @param message DeleteNetworkMembershipResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IDeleteNetworkMembershipResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a DeleteNetworkMembershipResponse message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns DeleteNetworkMembershipResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): DeleteNetworkMembershipResponse;

    /**
     * Decodes a DeleteNetworkMembershipResponse message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns DeleteNetworkMembershipResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): DeleteNetworkMembershipResponse;

    /**
     * Verifies a DeleteNetworkMembershipResponse message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a DeleteNetworkMembershipResponse message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns DeleteNetworkMembershipResponse
     */
    public static fromObject(object: { [k: string]: any }): DeleteNetworkMembershipResponse;

    /**
     * Creates a plain object from a DeleteNetworkMembershipResponse message. Also converts values to other types if specified.
     * @param message DeleteNetworkMembershipResponse
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: DeleteNetworkMembershipResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this DeleteNetworkMembershipResponse to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a StorageKey. */
export interface IStorageKey {

    /** StorageKey kdfType */
    kdfType?: (KDFType|null);

    /** StorageKey pbkdf2Options */
    pbkdf2Options?: (StorageKey.IPBKDF2Options|null);
}

/** Represents a StorageKey. */
export class StorageKey implements IStorageKey {

    /**
     * Constructs a new StorageKey.
     * @param [properties] Properties to set
     */
    constructor(properties?: IStorageKey);

    /** StorageKey kdfType. */
    public kdfType: KDFType;

    /** StorageKey pbkdf2Options. */
    public pbkdf2Options?: (StorageKey.IPBKDF2Options|null);

    /** StorageKey kdfOptions. */
    public kdfOptions?: "pbkdf2Options";

    /**
     * Creates a new StorageKey instance using the specified properties.
     * @param [properties] Properties to set
     * @returns StorageKey instance
     */
    public static create(properties?: IStorageKey): StorageKey;

    /**
     * Encodes the specified StorageKey message. Does not implicitly {@link StorageKey.verify|verify} messages.
     * @param message StorageKey message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IStorageKey, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified StorageKey message, length delimited. Does not implicitly {@link StorageKey.verify|verify} messages.
     * @param message StorageKey message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IStorageKey, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a StorageKey message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns StorageKey
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): StorageKey;

    /**
     * Decodes a StorageKey message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns StorageKey
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): StorageKey;

    /**
     * Verifies a StorageKey message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a StorageKey message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns StorageKey
     */
    public static fromObject(object: { [k: string]: any }): StorageKey;

    /**
     * Creates a plain object from a StorageKey message. Also converts values to other types if specified.
     * @param message StorageKey
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: StorageKey, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this StorageKey to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

export namespace StorageKey {

    /** Properties of a PBKDF2Options. */
    interface IPBKDF2Options {

        /** PBKDF2Options iterations */
        iterations?: (number|null);

        /** PBKDF2Options keySize */
        keySize?: (number|null);

        /** PBKDF2Options salt */
        salt?: (Uint8Array|null);
    }

    /** Represents a PBKDF2Options. */
    class PBKDF2Options implements IPBKDF2Options {

        /**
         * Constructs a new PBKDF2Options.
         * @param [properties] Properties to set
         */
        constructor(properties?: StorageKey.IPBKDF2Options);

        /** PBKDF2Options iterations. */
        public iterations: number;

        /** PBKDF2Options keySize. */
        public keySize: number;

        /** PBKDF2Options salt. */
        public salt: Uint8Array;

        /**
         * Creates a new PBKDF2Options instance using the specified properties.
         * @param [properties] Properties to set
         * @returns PBKDF2Options instance
         */
        public static create(properties?: StorageKey.IPBKDF2Options): StorageKey.PBKDF2Options;

        /**
         * Encodes the specified PBKDF2Options message. Does not implicitly {@link StorageKey.PBKDF2Options.verify|verify} messages.
         * @param message PBKDF2Options message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: StorageKey.IPBKDF2Options, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified PBKDF2Options message, length delimited. Does not implicitly {@link StorageKey.PBKDF2Options.verify|verify} messages.
         * @param message PBKDF2Options message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: StorageKey.IPBKDF2Options, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a PBKDF2Options message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns PBKDF2Options
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): StorageKey.PBKDF2Options;

        /**
         * Decodes a PBKDF2Options message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns PBKDF2Options
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): StorageKey.PBKDF2Options;

        /**
         * Verifies a PBKDF2Options message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a PBKDF2Options message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns PBKDF2Options
         */
        public static fromObject(object: { [k: string]: any }): StorageKey.PBKDF2Options;

        /**
         * Creates a plain object from a PBKDF2Options message. Also converts values to other types if specified.
         * @param message PBKDF2Options
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: StorageKey.PBKDF2Options, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this PBKDF2Options to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }
}

/** Properties of a Key. */
export interface IKey {

    /** Key type */
    type?: (KeyType|null);

    /** Key private */
    "private"?: (Uint8Array|null);

    /** Key public */
    "public"?: (Uint8Array|null);
}

/** Represents a Key. */
export class Key implements IKey {

    /**
     * Constructs a new Key.
     * @param [properties] Properties to set
     */
    constructor(properties?: IKey);

    /** Key type. */
    public type: KeyType;

    /** Key private. */
    public private: Uint8Array;

    /** Key public. */
    public public: Uint8Array;

    /**
     * Creates a new Key instance using the specified properties.
     * @param [properties] Properties to set
     * @returns Key instance
     */
    public static create(properties?: IKey): Key;

    /**
     * Encodes the specified Key message. Does not implicitly {@link Key.verify|verify} messages.
     * @param message Key message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IKey, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified Key message, length delimited. Does not implicitly {@link Key.verify|verify} messages.
     * @param message Key message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IKey, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a Key message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns Key
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): Key;

    /**
     * Decodes a Key message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns Key
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): Key;

    /**
     * Verifies a Key message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a Key message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns Key
     */
    public static fromObject(object: { [k: string]: any }): Key;

    /**
     * Creates a plain object from a Key message. Also converts values to other types if specified.
     * @param message Key
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: Key, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this Key to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a CertificateRequest. */
export interface ICertificateRequest {

    /** CertificateRequest key */
    key?: (Uint8Array|null);

    /** CertificateRequest keyType */
    keyType?: (KeyType|null);

    /** CertificateRequest keyUsage */
    keyUsage?: (number|null);

    /** CertificateRequest subject */
    subject?: (string|null);

    /** CertificateRequest signature */
    signature?: (Uint8Array|null);
}

/** Represents a CertificateRequest. */
export class CertificateRequest implements ICertificateRequest {

    /**
     * Constructs a new CertificateRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: ICertificateRequest);

    /** CertificateRequest key. */
    public key: Uint8Array;

    /** CertificateRequest keyType. */
    public keyType: KeyType;

    /** CertificateRequest keyUsage. */
    public keyUsage: number;

    /** CertificateRequest subject. */
    public subject: string;

    /** CertificateRequest signature. */
    public signature: Uint8Array;

    /**
     * Creates a new CertificateRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns CertificateRequest instance
     */
    public static create(properties?: ICertificateRequest): CertificateRequest;

    /**
     * Encodes the specified CertificateRequest message. Does not implicitly {@link CertificateRequest.verify|verify} messages.
     * @param message CertificateRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: ICertificateRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified CertificateRequest message, length delimited. Does not implicitly {@link CertificateRequest.verify|verify} messages.
     * @param message CertificateRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: ICertificateRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a CertificateRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns CertificateRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): CertificateRequest;

    /**
     * Decodes a CertificateRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns CertificateRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): CertificateRequest;

    /**
     * Verifies a CertificateRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a CertificateRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns CertificateRequest
     */
    public static fromObject(object: { [k: string]: any }): CertificateRequest;

    /**
     * Creates a plain object from a CertificateRequest message. Also converts values to other types if specified.
     * @param message CertificateRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: CertificateRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this CertificateRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a Certificate. */
export interface ICertificate {

    /** Certificate key */
    key?: (Uint8Array|null);

    /** Certificate keyType */
    keyType?: (KeyType|null);

    /** Certificate keyUsage */
    keyUsage?: (number|null);

    /** Certificate subject */
    subject?: (string|null);

    /** Certificate notBefore */
    notBefore?: (number|null);

    /** Certificate notAfter */
    notAfter?: (number|null);

    /** Certificate serialNumber */
    serialNumber?: (Uint8Array|null);

    /** Certificate signature */
    signature?: (Uint8Array|null);

    /** Certificate parent */
    parent?: (ICertificate|null);
}

/** Represents a Certificate. */
export class Certificate implements ICertificate {

    /**
     * Constructs a new Certificate.
     * @param [properties] Properties to set
     */
    constructor(properties?: ICertificate);

    /** Certificate key. */
    public key: Uint8Array;

    /** Certificate keyType. */
    public keyType: KeyType;

    /** Certificate keyUsage. */
    public keyUsage: number;

    /** Certificate subject. */
    public subject: string;

    /** Certificate notBefore. */
    public notBefore: number;

    /** Certificate notAfter. */
    public notAfter: number;

    /** Certificate serialNumber. */
    public serialNumber: Uint8Array;

    /** Certificate signature. */
    public signature: Uint8Array;

    /** Certificate parent. */
    public parent?: (ICertificate|null);

    /** Certificate parentOneof. */
    public parentOneof?: "parent";

    /**
     * Creates a new Certificate instance using the specified properties.
     * @param [properties] Properties to set
     * @returns Certificate instance
     */
    public static create(properties?: ICertificate): Certificate;

    /**
     * Encodes the specified Certificate message. Does not implicitly {@link Certificate.verify|verify} messages.
     * @param message Certificate message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: ICertificate, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified Certificate message, length delimited. Does not implicitly {@link Certificate.verify|verify} messages.
     * @param message Certificate message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: ICertificate, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a Certificate message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns Certificate
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): Certificate;

    /**
     * Decodes a Certificate message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns Certificate
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): Certificate;

    /**
     * Verifies a Certificate message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a Certificate message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns Certificate
     */
    public static fromObject(object: { [k: string]: any }): Certificate;

    /**
     * Creates a plain object from a Certificate message. Also converts values to other types if specified.
     * @param message Certificate
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: Certificate, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this Certificate to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a Profile. */
export interface IProfile {

    /** Profile id */
    id?: (number|null);

    /** Profile name */
    name?: (string|null);

    /** Profile secret */
    secret?: (Uint8Array|null);

    /** Profile key */
    key?: (IKey|null);

    /** Profile networks */
    networks?: (INetwork[]|null);

    /** Profile networkMemberships */
    networkMemberships?: (INetworkMembership[]|null);
}

/** Represents a Profile. */
export class Profile implements IProfile {

    /**
     * Constructs a new Profile.
     * @param [properties] Properties to set
     */
    constructor(properties?: IProfile);

    /** Profile id. */
    public id: number;

    /** Profile name. */
    public name: string;

    /** Profile secret. */
    public secret: Uint8Array;

    /** Profile key. */
    public key?: (IKey|null);

    /** Profile networks. */
    public networks: INetwork[];

    /** Profile networkMemberships. */
    public networkMemberships: INetworkMembership[];

    /**
     * Creates a new Profile instance using the specified properties.
     * @param [properties] Properties to set
     * @returns Profile instance
     */
    public static create(properties?: IProfile): Profile;

    /**
     * Encodes the specified Profile message. Does not implicitly {@link Profile.verify|verify} messages.
     * @param message Profile message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IProfile, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified Profile message, length delimited. Does not implicitly {@link Profile.verify|verify} messages.
     * @param message Profile message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IProfile, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a Profile message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns Profile
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): Profile;

    /**
     * Decodes a Profile message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns Profile
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): Profile;

    /**
     * Verifies a Profile message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a Profile message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns Profile
     */
    public static fromObject(object: { [k: string]: any }): Profile;

    /**
     * Creates a plain object from a Profile message. Also converts values to other types if specified.
     * @param message Profile
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: Profile, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this Profile to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a ProfileSummary. */
export interface IProfileSummary {

    /** ProfileSummary id */
    id?: (number|null);

    /** ProfileSummary name */
    name?: (string|null);
}

/** Represents a ProfileSummary. */
export class ProfileSummary implements IProfileSummary {

    /**
     * Constructs a new ProfileSummary.
     * @param [properties] Properties to set
     */
    constructor(properties?: IProfileSummary);

    /** ProfileSummary id. */
    public id: number;

    /** ProfileSummary name. */
    public name: string;

    /**
     * Creates a new ProfileSummary instance using the specified properties.
     * @param [properties] Properties to set
     * @returns ProfileSummary instance
     */
    public static create(properties?: IProfileSummary): ProfileSummary;

    /**
     * Encodes the specified ProfileSummary message. Does not implicitly {@link ProfileSummary.verify|verify} messages.
     * @param message ProfileSummary message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IProfileSummary, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified ProfileSummary message, length delimited. Does not implicitly {@link ProfileSummary.verify|verify} messages.
     * @param message ProfileSummary message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IProfileSummary, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a ProfileSummary message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns ProfileSummary
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): ProfileSummary;

    /**
     * Decodes a ProfileSummary message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns ProfileSummary
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): ProfileSummary;

    /**
     * Verifies a ProfileSummary message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a ProfileSummary message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns ProfileSummary
     */
    public static fromObject(object: { [k: string]: any }): ProfileSummary;

    /**
     * Creates a plain object from a ProfileSummary message. Also converts values to other types if specified.
     * @param message ProfileSummary
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: ProfileSummary, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this ProfileSummary to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a Network. */
export interface INetwork {

    /** Network id */
    id?: (number|null);

    /** Network name */
    name?: (string|null);

    /** Network key */
    key?: (IKey|null);

    /** Network certificate */
    certificate?: (ICertificate|null);
}

/** Represents a Network. */
export class Network implements INetwork {

    /**
     * Constructs a new Network.
     * @param [properties] Properties to set
     */
    constructor(properties?: INetwork);

    /** Network id. */
    public id: number;

    /** Network name. */
    public name: string;

    /** Network key. */
    public key?: (IKey|null);

    /** Network certificate. */
    public certificate?: (ICertificate|null);

    /**
     * Creates a new Network instance using the specified properties.
     * @param [properties] Properties to set
     * @returns Network instance
     */
    public static create(properties?: INetwork): Network;

    /**
     * Encodes the specified Network message. Does not implicitly {@link Network.verify|verify} messages.
     * @param message Network message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: INetwork, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified Network message, length delimited. Does not implicitly {@link Network.verify|verify} messages.
     * @param message Network message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: INetwork, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a Network message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns Network
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): Network;

    /**
     * Decodes a Network message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns Network
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): Network;

    /**
     * Verifies a Network message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a Network message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns Network
     */
    public static fromObject(object: { [k: string]: any }): Network;

    /**
     * Creates a plain object from a Network message. Also converts values to other types if specified.
     * @param message Network
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: Network, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this Network to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a NetworkMembership. */
export interface INetworkMembership {

    /** NetworkMembership id */
    id?: (number|null);

    /** NetworkMembership createdAt */
    createdAt?: (number|null);

    /** NetworkMembership name */
    name?: (string|null);

    /** NetworkMembership caCertificate */
    caCertificate?: (ICertificate|null);

    /** NetworkMembership certificate */
    certificate?: (ICertificate|null);

    /** NetworkMembership lastSeenAt */
    lastSeenAt?: (number|null);
}

/** Represents a NetworkMembership. */
export class NetworkMembership implements INetworkMembership {

    /**
     * Constructs a new NetworkMembership.
     * @param [properties] Properties to set
     */
    constructor(properties?: INetworkMembership);

    /** NetworkMembership id. */
    public id: number;

    /** NetworkMembership createdAt. */
    public createdAt: number;

    /** NetworkMembership name. */
    public name: string;

    /** NetworkMembership caCertificate. */
    public caCertificate?: (ICertificate|null);

    /** NetworkMembership certificate. */
    public certificate?: (ICertificate|null);

    /** NetworkMembership lastSeenAt. */
    public lastSeenAt: number;

    /**
     * Creates a new NetworkMembership instance using the specified properties.
     * @param [properties] Properties to set
     * @returns NetworkMembership instance
     */
    public static create(properties?: INetworkMembership): NetworkMembership;

    /**
     * Encodes the specified NetworkMembership message. Does not implicitly {@link NetworkMembership.verify|verify} messages.
     * @param message NetworkMembership message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: INetworkMembership, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified NetworkMembership message, length delimited. Does not implicitly {@link NetworkMembership.verify|verify} messages.
     * @param message NetworkMembership message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: INetworkMembership, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a NetworkMembership message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns NetworkMembership
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): NetworkMembership;

    /**
     * Decodes a NetworkMembership message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns NetworkMembership
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): NetworkMembership;

    /**
     * Verifies a NetworkMembership message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a NetworkMembership message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns NetworkMembership
     */
    public static fromObject(object: { [k: string]: any }): NetworkMembership;

    /**
     * Creates a plain object from a NetworkMembership message. Also converts values to other types if specified.
     * @param message NetworkMembership
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: NetworkMembership, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this NetworkMembership to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a CreateNetworkInvitationRequest. */
export interface ICreateNetworkInvitationRequest {

    /** CreateNetworkInvitationRequest signingKey */
    signingKey?: (IKey|null);

    /** CreateNetworkInvitationRequest signingCert */
    signingCert?: (ICertificate|null);

    /** CreateNetworkInvitationRequest networkName */
    networkName?: (string|null);
}

/** Represents a CreateNetworkInvitationRequest. */
export class CreateNetworkInvitationRequest implements ICreateNetworkInvitationRequest {

    /**
     * Constructs a new CreateNetworkInvitationRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: ICreateNetworkInvitationRequest);

    /** CreateNetworkInvitationRequest signingKey. */
    public signingKey?: (IKey|null);

    /** CreateNetworkInvitationRequest signingCert. */
    public signingCert?: (ICertificate|null);

    /** CreateNetworkInvitationRequest networkName. */
    public networkName: string;

    /**
     * Creates a new CreateNetworkInvitationRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns CreateNetworkInvitationRequest instance
     */
    public static create(properties?: ICreateNetworkInvitationRequest): CreateNetworkInvitationRequest;

    /**
     * Encodes the specified CreateNetworkInvitationRequest message. Does not implicitly {@link CreateNetworkInvitationRequest.verify|verify} messages.
     * @param message CreateNetworkInvitationRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: ICreateNetworkInvitationRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified CreateNetworkInvitationRequest message, length delimited. Does not implicitly {@link CreateNetworkInvitationRequest.verify|verify} messages.
     * @param message CreateNetworkInvitationRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: ICreateNetworkInvitationRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a CreateNetworkInvitationRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns CreateNetworkInvitationRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): CreateNetworkInvitationRequest;

    /**
     * Decodes a CreateNetworkInvitationRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns CreateNetworkInvitationRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): CreateNetworkInvitationRequest;

    /**
     * Verifies a CreateNetworkInvitationRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a CreateNetworkInvitationRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns CreateNetworkInvitationRequest
     */
    public static fromObject(object: { [k: string]: any }): CreateNetworkInvitationRequest;

    /**
     * Creates a plain object from a CreateNetworkInvitationRequest message. Also converts values to other types if specified.
     * @param message CreateNetworkInvitationRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: CreateNetworkInvitationRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this CreateNetworkInvitationRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a CreateNetworkInvitationResponse. */
export interface ICreateNetworkInvitationResponse {

    /** CreateNetworkInvitationResponse invitation */
    invitation?: (IInvitation|null);

    /** CreateNetworkInvitationResponse invitationB64 */
    invitationB64?: (string|null);

    /** CreateNetworkInvitationResponse invitationBytes */
    invitationBytes?: (Uint8Array|null);
}

/** Represents a CreateNetworkInvitationResponse. */
export class CreateNetworkInvitationResponse implements ICreateNetworkInvitationResponse {

    /**
     * Constructs a new CreateNetworkInvitationResponse.
     * @param [properties] Properties to set
     */
    constructor(properties?: ICreateNetworkInvitationResponse);

    /** CreateNetworkInvitationResponse invitation. */
    public invitation?: (IInvitation|null);

    /** CreateNetworkInvitationResponse invitationB64. */
    public invitationB64: string;

    /** CreateNetworkInvitationResponse invitationBytes. */
    public invitationBytes: Uint8Array;

    /**
     * Creates a new CreateNetworkInvitationResponse instance using the specified properties.
     * @param [properties] Properties to set
     * @returns CreateNetworkInvitationResponse instance
     */
    public static create(properties?: ICreateNetworkInvitationResponse): CreateNetworkInvitationResponse;

    /**
     * Encodes the specified CreateNetworkInvitationResponse message. Does not implicitly {@link CreateNetworkInvitationResponse.verify|verify} messages.
     * @param message CreateNetworkInvitationResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: ICreateNetworkInvitationResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified CreateNetworkInvitationResponse message, length delimited. Does not implicitly {@link CreateNetworkInvitationResponse.verify|verify} messages.
     * @param message CreateNetworkInvitationResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: ICreateNetworkInvitationResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a CreateNetworkInvitationResponse message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns CreateNetworkInvitationResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): CreateNetworkInvitationResponse;

    /**
     * Decodes a CreateNetworkInvitationResponse message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns CreateNetworkInvitationResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): CreateNetworkInvitationResponse;

    /**
     * Verifies a CreateNetworkInvitationResponse message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a CreateNetworkInvitationResponse message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns CreateNetworkInvitationResponse
     */
    public static fromObject(object: { [k: string]: any }): CreateNetworkInvitationResponse;

    /**
     * Creates a plain object from a CreateNetworkInvitationResponse message. Also converts values to other types if specified.
     * @param message CreateNetworkInvitationResponse
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: CreateNetworkInvitationResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this CreateNetworkInvitationResponse to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of an Invitation. */
export interface IInvitation {

    /** Invitation version */
    version?: (number|null);

    /** Invitation data */
    data?: (Uint8Array|null);
}

/** Represents an Invitation. */
export class Invitation implements IInvitation {

    /**
     * Constructs a new Invitation.
     * @param [properties] Properties to set
     */
    constructor(properties?: IInvitation);

    /** Invitation version. */
    public version: number;

    /** Invitation data. */
    public data: Uint8Array;

    /**
     * Creates a new Invitation instance using the specified properties.
     * @param [properties] Properties to set
     * @returns Invitation instance
     */
    public static create(properties?: IInvitation): Invitation;

    /**
     * Encodes the specified Invitation message. Does not implicitly {@link Invitation.verify|verify} messages.
     * @param message Invitation message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IInvitation, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified Invitation message, length delimited. Does not implicitly {@link Invitation.verify|verify} messages.
     * @param message Invitation message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IInvitation, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes an Invitation message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns Invitation
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): Invitation;

    /**
     * Decodes an Invitation message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns Invitation
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): Invitation;

    /**
     * Verifies an Invitation message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates an Invitation message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns Invitation
     */
    public static fromObject(object: { [k: string]: any }): Invitation;

    /**
     * Creates a plain object from an Invitation message. Also converts values to other types if specified.
     * @param message Invitation
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: Invitation, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this Invitation to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of an InvitationV0. */
export interface IInvitationV0 {

    /** InvitationV0 key */
    key?: (IKey|null);

    /** InvitationV0 certificate */
    certificate?: (ICertificate|null);

    /** InvitationV0 networkName */
    networkName?: (string|null);
}

/** Represents an InvitationV0. */
export class InvitationV0 implements IInvitationV0 {

    /**
     * Constructs a new InvitationV0.
     * @param [properties] Properties to set
     */
    constructor(properties?: IInvitationV0);

    /** InvitationV0 key. */
    public key?: (IKey|null);

    /** InvitationV0 certificate. */
    public certificate?: (ICertificate|null);

    /** InvitationV0 networkName. */
    public networkName: string;

    /**
     * Creates a new InvitationV0 instance using the specified properties.
     * @param [properties] Properties to set
     * @returns InvitationV0 instance
     */
    public static create(properties?: IInvitationV0): InvitationV0;

    /**
     * Encodes the specified InvitationV0 message. Does not implicitly {@link InvitationV0.verify|verify} messages.
     * @param message InvitationV0 message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IInvitationV0, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified InvitationV0 message, length delimited. Does not implicitly {@link InvitationV0.verify|verify} messages.
     * @param message InvitationV0 message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IInvitationV0, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes an InvitationV0 message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns InvitationV0
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): InvitationV0;

    /**
     * Decodes an InvitationV0 message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns InvitationV0
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): InvitationV0;

    /**
     * Verifies an InvitationV0 message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates an InvitationV0 message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns InvitationV0
     */
    public static fromObject(object: { [k: string]: any }): InvitationV0;

    /**
     * Creates a plain object from an InvitationV0 message. Also converts values to other types if specified.
     * @param message InvitationV0
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: InvitationV0, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this InvitationV0 to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a CreateNetworkMembershipFromInvitationRequest. */
export interface ICreateNetworkMembershipFromInvitationRequest {

    /** CreateNetworkMembershipFromInvitationRequest invitationB64 */
    invitationB64?: (string|null);

    /** CreateNetworkMembershipFromInvitationRequest invitationBytes */
    invitationBytes?: (Uint8Array|null);
}

/** Represents a CreateNetworkMembershipFromInvitationRequest. */
export class CreateNetworkMembershipFromInvitationRequest implements ICreateNetworkMembershipFromInvitationRequest {

    /**
     * Constructs a new CreateNetworkMembershipFromInvitationRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: ICreateNetworkMembershipFromInvitationRequest);

    /** CreateNetworkMembershipFromInvitationRequest invitationB64. */
    public invitationB64: string;

    /** CreateNetworkMembershipFromInvitationRequest invitationBytes. */
    public invitationBytes: Uint8Array;

    /** CreateNetworkMembershipFromInvitationRequest invitation. */
    public invitation?: ("invitationB64"|"invitationBytes");

    /**
     * Creates a new CreateNetworkMembershipFromInvitationRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns CreateNetworkMembershipFromInvitationRequest instance
     */
    public static create(properties?: ICreateNetworkMembershipFromInvitationRequest): CreateNetworkMembershipFromInvitationRequest;

    /**
     * Encodes the specified CreateNetworkMembershipFromInvitationRequest message. Does not implicitly {@link CreateNetworkMembershipFromInvitationRequest.verify|verify} messages.
     * @param message CreateNetworkMembershipFromInvitationRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: ICreateNetworkMembershipFromInvitationRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified CreateNetworkMembershipFromInvitationRequest message, length delimited. Does not implicitly {@link CreateNetworkMembershipFromInvitationRequest.verify|verify} messages.
     * @param message CreateNetworkMembershipFromInvitationRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: ICreateNetworkMembershipFromInvitationRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a CreateNetworkMembershipFromInvitationRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns CreateNetworkMembershipFromInvitationRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): CreateNetworkMembershipFromInvitationRequest;

    /**
     * Decodes a CreateNetworkMembershipFromInvitationRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns CreateNetworkMembershipFromInvitationRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): CreateNetworkMembershipFromInvitationRequest;

    /**
     * Verifies a CreateNetworkMembershipFromInvitationRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a CreateNetworkMembershipFromInvitationRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns CreateNetworkMembershipFromInvitationRequest
     */
    public static fromObject(object: { [k: string]: any }): CreateNetworkMembershipFromInvitationRequest;

    /**
     * Creates a plain object from a CreateNetworkMembershipFromInvitationRequest message. Also converts values to other types if specified.
     * @param message CreateNetworkMembershipFromInvitationRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: CreateNetworkMembershipFromInvitationRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this CreateNetworkMembershipFromInvitationRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a CreateNetworkMembershipFromInvitationResponse. */
export interface ICreateNetworkMembershipFromInvitationResponse {

    /** CreateNetworkMembershipFromInvitationResponse membership */
    membership?: (INetworkMembership|null);
}

/** Represents a CreateNetworkMembershipFromInvitationResponse. */
export class CreateNetworkMembershipFromInvitationResponse implements ICreateNetworkMembershipFromInvitationResponse {

    /**
     * Constructs a new CreateNetworkMembershipFromInvitationResponse.
     * @param [properties] Properties to set
     */
    constructor(properties?: ICreateNetworkMembershipFromInvitationResponse);

    /** CreateNetworkMembershipFromInvitationResponse membership. */
    public membership?: (INetworkMembership|null);

    /**
     * Creates a new CreateNetworkMembershipFromInvitationResponse instance using the specified properties.
     * @param [properties] Properties to set
     * @returns CreateNetworkMembershipFromInvitationResponse instance
     */
    public static create(properties?: ICreateNetworkMembershipFromInvitationResponse): CreateNetworkMembershipFromInvitationResponse;

    /**
     * Encodes the specified CreateNetworkMembershipFromInvitationResponse message. Does not implicitly {@link CreateNetworkMembershipFromInvitationResponse.verify|verify} messages.
     * @param message CreateNetworkMembershipFromInvitationResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: ICreateNetworkMembershipFromInvitationResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified CreateNetworkMembershipFromInvitationResponse message, length delimited. Does not implicitly {@link CreateNetworkMembershipFromInvitationResponse.verify|verify} messages.
     * @param message CreateNetworkMembershipFromInvitationResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: ICreateNetworkMembershipFromInvitationResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a CreateNetworkMembershipFromInvitationResponse message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns CreateNetworkMembershipFromInvitationResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): CreateNetworkMembershipFromInvitationResponse;

    /**
     * Decodes a CreateNetworkMembershipFromInvitationResponse message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns CreateNetworkMembershipFromInvitationResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): CreateNetworkMembershipFromInvitationResponse;

    /**
     * Verifies a CreateNetworkMembershipFromInvitationResponse message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a CreateNetworkMembershipFromInvitationResponse message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns CreateNetworkMembershipFromInvitationResponse
     */
    public static fromObject(object: { [k: string]: any }): CreateNetworkMembershipFromInvitationResponse;

    /**
     * Creates a plain object from a CreateNetworkMembershipFromInvitationResponse message. Also converts values to other types if specified.
     * @param message CreateNetworkMembershipFromInvitationResponse
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: CreateNetworkMembershipFromInvitationResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this CreateNetworkMembershipFromInvitationResponse to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a Mutex. */
export interface IMutex {

    /** Mutex eol */
    eol?: (number|null);

    /** Mutex token */
    token?: (Uint8Array|null);
}

/** Represents a Mutex. */
export class Mutex implements IMutex {

    /**
     * Constructs a new Mutex.
     * @param [properties] Properties to set
     */
    constructor(properties?: IMutex);

    /** Mutex eol. */
    public eol: number;

    /** Mutex token. */
    public token: Uint8Array;

    /**
     * Creates a new Mutex instance using the specified properties.
     * @param [properties] Properties to set
     * @returns Mutex instance
     */
    public static create(properties?: IMutex): Mutex;

    /**
     * Encodes the specified Mutex message. Does not implicitly {@link Mutex.verify|verify} messages.
     * @param message Mutex message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IMutex, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified Mutex message, length delimited. Does not implicitly {@link Mutex.verify|verify} messages.
     * @param message Mutex message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IMutex, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a Mutex message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns Mutex
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): Mutex;

    /**
     * Decodes a Mutex message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns Mutex
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): Mutex;

    /**
     * Verifies a Mutex message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a Mutex message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns Mutex
     */
    public static fromObject(object: { [k: string]: any }): Mutex;

    /**
     * Creates a plain object from a Mutex message. Also converts values to other types if specified.
     * @param message Mutex
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: Mutex, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this Mutex to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a PProfRequest. */
export interface IPProfRequest {

    /** PProfRequest name */
    name?: (string|null);

    /** PProfRequest debug */
    debug?: (boolean|null);

    /** PProfRequest gc */
    gc?: (boolean|null);
}

/** Represents a PProfRequest. */
export class PProfRequest implements IPProfRequest {

    /**
     * Constructs a new PProfRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: IPProfRequest);

    /** PProfRequest name. */
    public name: string;

    /** PProfRequest debug. */
    public debug: boolean;

    /** PProfRequest gc. */
    public gc: boolean;

    /**
     * Creates a new PProfRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns PProfRequest instance
     */
    public static create(properties?: IPProfRequest): PProfRequest;

    /**
     * Encodes the specified PProfRequest message. Does not implicitly {@link PProfRequest.verify|verify} messages.
     * @param message PProfRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IPProfRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified PProfRequest message, length delimited. Does not implicitly {@link PProfRequest.verify|verify} messages.
     * @param message PProfRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IPProfRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a PProfRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns PProfRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): PProfRequest;

    /**
     * Decodes a PProfRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns PProfRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): PProfRequest;

    /**
     * Verifies a PProfRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a PProfRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns PProfRequest
     */
    public static fromObject(object: { [k: string]: any }): PProfRequest;

    /**
     * Creates a plain object from a PProfRequest message. Also converts values to other types if specified.
     * @param message PProfRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: PProfRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this PProfRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a PProfResponse. */
export interface IPProfResponse {

    /** PProfResponse name */
    name?: (string|null);

    /** PProfResponse data */
    data?: (Uint8Array|null);
}

/** Represents a PProfResponse. */
export class PProfResponse implements IPProfResponse {

    /**
     * Constructs a new PProfResponse.
     * @param [properties] Properties to set
     */
    constructor(properties?: IPProfResponse);

    /** PProfResponse name. */
    public name: string;

    /** PProfResponse data. */
    public data: Uint8Array;

    /**
     * Creates a new PProfResponse instance using the specified properties.
     * @param [properties] Properties to set
     * @returns PProfResponse instance
     */
    public static create(properties?: IPProfResponse): PProfResponse;

    /**
     * Encodes the specified PProfResponse message. Does not implicitly {@link PProfResponse.verify|verify} messages.
     * @param message PProfResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IPProfResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified PProfResponse message, length delimited. Does not implicitly {@link PProfResponse.verify|verify} messages.
     * @param message PProfResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IPProfResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a PProfResponse message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns PProfResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): PProfResponse;

    /**
     * Decodes a PProfResponse message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns PProfResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): PProfResponse;

    /**
     * Verifies a PProfResponse message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a PProfResponse message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns PProfResponse
     */
    public static fromObject(object: { [k: string]: any }): PProfResponse;

    /**
     * Creates a plain object from a PProfResponse message. Also converts values to other types if specified.
     * @param message PProfResponse
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: PProfResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this PProfResponse to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a ReadMetricsRequest. */
export interface IReadMetricsRequest {

    /** ReadMetricsRequest format */
    format?: (MetricsFormat|null);
}

/** Represents a ReadMetricsRequest. */
export class ReadMetricsRequest implements IReadMetricsRequest {

    /**
     * Constructs a new ReadMetricsRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: IReadMetricsRequest);

    /** ReadMetricsRequest format. */
    public format: MetricsFormat;

    /**
     * Creates a new ReadMetricsRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns ReadMetricsRequest instance
     */
    public static create(properties?: IReadMetricsRequest): ReadMetricsRequest;

    /**
     * Encodes the specified ReadMetricsRequest message. Does not implicitly {@link ReadMetricsRequest.verify|verify} messages.
     * @param message ReadMetricsRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IReadMetricsRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified ReadMetricsRequest message, length delimited. Does not implicitly {@link ReadMetricsRequest.verify|verify} messages.
     * @param message ReadMetricsRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IReadMetricsRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a ReadMetricsRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns ReadMetricsRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): ReadMetricsRequest;

    /**
     * Decodes a ReadMetricsRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns ReadMetricsRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): ReadMetricsRequest;

    /**
     * Verifies a ReadMetricsRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a ReadMetricsRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns ReadMetricsRequest
     */
    public static fromObject(object: { [k: string]: any }): ReadMetricsRequest;

    /**
     * Creates a plain object from a ReadMetricsRequest message. Also converts values to other types if specified.
     * @param message ReadMetricsRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: ReadMetricsRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this ReadMetricsRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a ReadMetricsResponse. */
export interface IReadMetricsResponse {

    /** ReadMetricsResponse data */
    data?: (Uint8Array|null);
}

/** Represents a ReadMetricsResponse. */
export class ReadMetricsResponse implements IReadMetricsResponse {

    /**
     * Constructs a new ReadMetricsResponse.
     * @param [properties] Properties to set
     */
    constructor(properties?: IReadMetricsResponse);

    /** ReadMetricsResponse data. */
    public data: Uint8Array;

    /**
     * Creates a new ReadMetricsResponse instance using the specified properties.
     * @param [properties] Properties to set
     * @returns ReadMetricsResponse instance
     */
    public static create(properties?: IReadMetricsResponse): ReadMetricsResponse;

    /**
     * Encodes the specified ReadMetricsResponse message. Does not implicitly {@link ReadMetricsResponse.verify|verify} messages.
     * @param message ReadMetricsResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IReadMetricsResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified ReadMetricsResponse message, length delimited. Does not implicitly {@link ReadMetricsResponse.verify|verify} messages.
     * @param message ReadMetricsResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IReadMetricsResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a ReadMetricsResponse message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns ReadMetricsResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): ReadMetricsResponse;

    /**
     * Decodes a ReadMetricsResponse message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns ReadMetricsResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): ReadMetricsResponse;

    /**
     * Verifies a ReadMetricsResponse message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a ReadMetricsResponse message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns ReadMetricsResponse
     */
    public static fromObject(object: { [k: string]: any }): ReadMetricsResponse;

    /**
     * Creates a plain object from a ReadMetricsResponse message. Also converts values to other types if specified.
     * @param message ReadMetricsResponse
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: ReadMetricsResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this ReadMetricsResponse to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a MonitorSwarmsRequest. */
export interface IMonitorSwarmsRequest {
}

/** Represents a MonitorSwarmsRequest. */
export class MonitorSwarmsRequest implements IMonitorSwarmsRequest {

    /**
     * Constructs a new MonitorSwarmsRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: IMonitorSwarmsRequest);

    /**
     * Creates a new MonitorSwarmsRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns MonitorSwarmsRequest instance
     */
    public static create(properties?: IMonitorSwarmsRequest): MonitorSwarmsRequest;

    /**
     * Encodes the specified MonitorSwarmsRequest message. Does not implicitly {@link MonitorSwarmsRequest.verify|verify} messages.
     * @param message MonitorSwarmsRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IMonitorSwarmsRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified MonitorSwarmsRequest message, length delimited. Does not implicitly {@link MonitorSwarmsRequest.verify|verify} messages.
     * @param message MonitorSwarmsRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IMonitorSwarmsRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a MonitorSwarmsRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns MonitorSwarmsRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): MonitorSwarmsRequest;

    /**
     * Decodes a MonitorSwarmsRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns MonitorSwarmsRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): MonitorSwarmsRequest;

    /**
     * Verifies a MonitorSwarmsRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a MonitorSwarmsRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns MonitorSwarmsRequest
     */
    public static fromObject(object: { [k: string]: any }): MonitorSwarmsRequest;

    /**
     * Creates a plain object from a MonitorSwarmsRequest message. Also converts values to other types if specified.
     * @param message MonitorSwarmsRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: MonitorSwarmsRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this MonitorSwarmsRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a MonitorSwarmsResponse. */
export interface IMonitorSwarmsResponse {

    /** MonitorSwarmsResponse type */
    type?: (SwarmEventType|null);

    /** MonitorSwarmsResponse id */
    id?: (string|null);
}

/** Represents a MonitorSwarmsResponse. */
export class MonitorSwarmsResponse implements IMonitorSwarmsResponse {

    /**
     * Constructs a new MonitorSwarmsResponse.
     * @param [properties] Properties to set
     */
    constructor(properties?: IMonitorSwarmsResponse);

    /** MonitorSwarmsResponse type. */
    public type: SwarmEventType;

    /** MonitorSwarmsResponse id. */
    public id: string;

    /**
     * Creates a new MonitorSwarmsResponse instance using the specified properties.
     * @param [properties] Properties to set
     * @returns MonitorSwarmsResponse instance
     */
    public static create(properties?: IMonitorSwarmsResponse): MonitorSwarmsResponse;

    /**
     * Encodes the specified MonitorSwarmsResponse message. Does not implicitly {@link MonitorSwarmsResponse.verify|verify} messages.
     * @param message MonitorSwarmsResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IMonitorSwarmsResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified MonitorSwarmsResponse message, length delimited. Does not implicitly {@link MonitorSwarmsResponse.verify|verify} messages.
     * @param message MonitorSwarmsResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IMonitorSwarmsResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a MonitorSwarmsResponse message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns MonitorSwarmsResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): MonitorSwarmsResponse;

    /**
     * Decodes a MonitorSwarmsResponse message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns MonitorSwarmsResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): MonitorSwarmsResponse;

    /**
     * Verifies a MonitorSwarmsResponse message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a MonitorSwarmsResponse message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns MonitorSwarmsResponse
     */
    public static fromObject(object: { [k: string]: any }): MonitorSwarmsResponse;

    /**
     * Creates a plain object from a MonitorSwarmsResponse message. Also converts values to other types if specified.
     * @param message MonitorSwarmsResponse
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: MonitorSwarmsResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this MonitorSwarmsResponse to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a BootstrapDHTRequest. */
export interface IBootstrapDHTRequest {

    /** BootstrapDHTRequest transportUris */
    transportUris?: (string[]|null);
}

/** Represents a BootstrapDHTRequest. */
export class BootstrapDHTRequest implements IBootstrapDHTRequest {

    /**
     * Constructs a new BootstrapDHTRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: IBootstrapDHTRequest);

    /** BootstrapDHTRequest transportUris. */
    public transportUris: string[];

    /**
     * Creates a new BootstrapDHTRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns BootstrapDHTRequest instance
     */
    public static create(properties?: IBootstrapDHTRequest): BootstrapDHTRequest;

    /**
     * Encodes the specified BootstrapDHTRequest message. Does not implicitly {@link BootstrapDHTRequest.verify|verify} messages.
     * @param message BootstrapDHTRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IBootstrapDHTRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified BootstrapDHTRequest message, length delimited. Does not implicitly {@link BootstrapDHTRequest.verify|verify} messages.
     * @param message BootstrapDHTRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IBootstrapDHTRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a BootstrapDHTRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns BootstrapDHTRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): BootstrapDHTRequest;

    /**
     * Decodes a BootstrapDHTRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns BootstrapDHTRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): BootstrapDHTRequest;

    /**
     * Verifies a BootstrapDHTRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a BootstrapDHTRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns BootstrapDHTRequest
     */
    public static fromObject(object: { [k: string]: any }): BootstrapDHTRequest;

    /**
     * Creates a plain object from a BootstrapDHTRequest message. Also converts values to other types if specified.
     * @param message BootstrapDHTRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: BootstrapDHTRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this BootstrapDHTRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a BootstrapDHTResponse. */
export interface IBootstrapDHTResponse {
}

/** Represents a BootstrapDHTResponse. */
export class BootstrapDHTResponse implements IBootstrapDHTResponse {

    /**
     * Constructs a new BootstrapDHTResponse.
     * @param [properties] Properties to set
     */
    constructor(properties?: IBootstrapDHTResponse);

    /**
     * Creates a new BootstrapDHTResponse instance using the specified properties.
     * @param [properties] Properties to set
     * @returns BootstrapDHTResponse instance
     */
    public static create(properties?: IBootstrapDHTResponse): BootstrapDHTResponse;

    /**
     * Encodes the specified BootstrapDHTResponse message. Does not implicitly {@link BootstrapDHTResponse.verify|verify} messages.
     * @param message BootstrapDHTResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IBootstrapDHTResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified BootstrapDHTResponse message, length delimited. Does not implicitly {@link BootstrapDHTResponse.verify|verify} messages.
     * @param message BootstrapDHTResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IBootstrapDHTResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a BootstrapDHTResponse message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns BootstrapDHTResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): BootstrapDHTResponse;

    /**
     * Decodes a BootstrapDHTResponse message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns BootstrapDHTResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): BootstrapDHTResponse;

    /**
     * Verifies a BootstrapDHTResponse message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a BootstrapDHTResponse message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns BootstrapDHTResponse
     */
    public static fromObject(object: { [k: string]: any }): BootstrapDHTResponse;

    /**
     * Creates a plain object from a BootstrapDHTResponse message. Also converts values to other types if specified.
     * @param message BootstrapDHTResponse
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: BootstrapDHTResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this BootstrapDHTResponse to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a NegotiateWRTCRequest. */
export interface INegotiateWRTCRequest {

    /** NegotiateWRTCRequest type */
    type?: (WRTCSDPType|null);

    /** NegotiateWRTCRequest sessionDescription */
    sessionDescription?: (string|null);
}

/** Represents a NegotiateWRTCRequest. */
export class NegotiateWRTCRequest implements INegotiateWRTCRequest {

    /**
     * Constructs a new NegotiateWRTCRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: INegotiateWRTCRequest);

    /** NegotiateWRTCRequest type. */
    public type: WRTCSDPType;

    /** NegotiateWRTCRequest sessionDescription. */
    public sessionDescription: string;

    /**
     * Creates a new NegotiateWRTCRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns NegotiateWRTCRequest instance
     */
    public static create(properties?: INegotiateWRTCRequest): NegotiateWRTCRequest;

    /**
     * Encodes the specified NegotiateWRTCRequest message. Does not implicitly {@link NegotiateWRTCRequest.verify|verify} messages.
     * @param message NegotiateWRTCRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: INegotiateWRTCRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified NegotiateWRTCRequest message, length delimited. Does not implicitly {@link NegotiateWRTCRequest.verify|verify} messages.
     * @param message NegotiateWRTCRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: INegotiateWRTCRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a NegotiateWRTCRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns NegotiateWRTCRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): NegotiateWRTCRequest;

    /**
     * Decodes a NegotiateWRTCRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns NegotiateWRTCRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): NegotiateWRTCRequest;

    /**
     * Verifies a NegotiateWRTCRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a NegotiateWRTCRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns NegotiateWRTCRequest
     */
    public static fromObject(object: { [k: string]: any }): NegotiateWRTCRequest;

    /**
     * Creates a plain object from a NegotiateWRTCRequest message. Also converts values to other types if specified.
     * @param message NegotiateWRTCRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: NegotiateWRTCRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this NegotiateWRTCRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a NegotiateWRTCResponse. */
export interface INegotiateWRTCResponse {

    /** NegotiateWRTCResponse candidate */
    candidate?: (string|null);
}

/** Represents a NegotiateWRTCResponse. */
export class NegotiateWRTCResponse implements INegotiateWRTCResponse {

    /**
     * Constructs a new NegotiateWRTCResponse.
     * @param [properties] Properties to set
     */
    constructor(properties?: INegotiateWRTCResponse);

    /** NegotiateWRTCResponse candidate. */
    public candidate: string;

    /**
     * Creates a new NegotiateWRTCResponse instance using the specified properties.
     * @param [properties] Properties to set
     * @returns NegotiateWRTCResponse instance
     */
    public static create(properties?: INegotiateWRTCResponse): NegotiateWRTCResponse;

    /**
     * Encodes the specified NegotiateWRTCResponse message. Does not implicitly {@link NegotiateWRTCResponse.verify|verify} messages.
     * @param message NegotiateWRTCResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: INegotiateWRTCResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified NegotiateWRTCResponse message, length delimited. Does not implicitly {@link NegotiateWRTCResponse.verify|verify} messages.
     * @param message NegotiateWRTCResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: INegotiateWRTCResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a NegotiateWRTCResponse message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns NegotiateWRTCResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): NegotiateWRTCResponse;

    /**
     * Decodes a NegotiateWRTCResponse message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns NegotiateWRTCResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): NegotiateWRTCResponse;

    /**
     * Verifies a NegotiateWRTCResponse message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a NegotiateWRTCResponse message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns NegotiateWRTCResponse
     */
    public static fromObject(object: { [k: string]: any }): NegotiateWRTCResponse;

    /**
     * Creates a plain object from a NegotiateWRTCResponse message. Also converts values to other types if specified.
     * @param message NegotiateWRTCResponse
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: NegotiateWRTCResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this NegotiateWRTCResponse to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a HashTableMessage. */
export interface IHashTableMessage {

    /** HashTableMessage publish */
    publish?: (HashTableMessage.IPublish|null);

    /** HashTableMessage unpublish */
    unpublish?: (HashTableMessage.IUnpublish|null);

    /** HashTableMessage getRequest */
    getRequest?: (HashTableMessage.IGetRequest|null);

    /** HashTableMessage getResponse */
    getResponse?: (HashTableMessage.IGetResponse|null);
}

/** Represents a HashTableMessage. */
export class HashTableMessage implements IHashTableMessage {

    /**
     * Constructs a new HashTableMessage.
     * @param [properties] Properties to set
     */
    constructor(properties?: IHashTableMessage);

    /** HashTableMessage publish. */
    public publish?: (HashTableMessage.IPublish|null);

    /** HashTableMessage unpublish. */
    public unpublish?: (HashTableMessage.IUnpublish|null);

    /** HashTableMessage getRequest. */
    public getRequest?: (HashTableMessage.IGetRequest|null);

    /** HashTableMessage getResponse. */
    public getResponse?: (HashTableMessage.IGetResponse|null);

    /** HashTableMessage body. */
    public body?: ("publish"|"unpublish"|"getRequest"|"getResponse");

    /**
     * Creates a new HashTableMessage instance using the specified properties.
     * @param [properties] Properties to set
     * @returns HashTableMessage instance
     */
    public static create(properties?: IHashTableMessage): HashTableMessage;

    /**
     * Encodes the specified HashTableMessage message. Does not implicitly {@link HashTableMessage.verify|verify} messages.
     * @param message HashTableMessage message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IHashTableMessage, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified HashTableMessage message, length delimited. Does not implicitly {@link HashTableMessage.verify|verify} messages.
     * @param message HashTableMessage message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IHashTableMessage, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a HashTableMessage message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns HashTableMessage
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): HashTableMessage;

    /**
     * Decodes a HashTableMessage message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns HashTableMessage
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): HashTableMessage;

    /**
     * Verifies a HashTableMessage message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a HashTableMessage message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns HashTableMessage
     */
    public static fromObject(object: { [k: string]: any }): HashTableMessage;

    /**
     * Creates a plain object from a HashTableMessage message. Also converts values to other types if specified.
     * @param message HashTableMessage
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: HashTableMessage, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this HashTableMessage to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

export namespace HashTableMessage {

    /** Properties of a Record. */
    interface IRecord {

        /** Record key */
        key?: (Uint8Array|null);

        /** Record salt */
        salt?: (Uint8Array|null);

        /** Record value */
        value?: (Uint8Array|null);

        /** Record timestamp */
        timestamp?: (number|null);

        /** Record signature */
        signature?: (Uint8Array|null);
    }

    /** Represents a Record. */
    class Record implements IRecord {

        /**
         * Constructs a new Record.
         * @param [properties] Properties to set
         */
        constructor(properties?: HashTableMessage.IRecord);

        /** Record key. */
        public key: Uint8Array;

        /** Record salt. */
        public salt: Uint8Array;

        /** Record value. */
        public value: Uint8Array;

        /** Record timestamp. */
        public timestamp: number;

        /** Record signature. */
        public signature: Uint8Array;

        /**
         * Creates a new Record instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Record instance
         */
        public static create(properties?: HashTableMessage.IRecord): HashTableMessage.Record;

        /**
         * Encodes the specified Record message. Does not implicitly {@link HashTableMessage.Record.verify|verify} messages.
         * @param message Record message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: HashTableMessage.IRecord, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Record message, length delimited. Does not implicitly {@link HashTableMessage.Record.verify|verify} messages.
         * @param message Record message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: HashTableMessage.IRecord, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a Record message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Record
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): HashTableMessage.Record;

        /**
         * Decodes a Record message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Record
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): HashTableMessage.Record;

        /**
         * Verifies a Record message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a Record message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Record
         */
        public static fromObject(object: { [k: string]: any }): HashTableMessage.Record;

        /**
         * Creates a plain object from a Record message. Also converts values to other types if specified.
         * @param message Record
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: HashTableMessage.Record, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Record to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }

    /** Properties of a Publish. */
    interface IPublish {

        /** Publish record */
        record?: (HashTableMessage.IRecord|null);
    }

    /** Represents a Publish. */
    class Publish implements IPublish {

        /**
         * Constructs a new Publish.
         * @param [properties] Properties to set
         */
        constructor(properties?: HashTableMessage.IPublish);

        /** Publish record. */
        public record?: (HashTableMessage.IRecord|null);

        /**
         * Creates a new Publish instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Publish instance
         */
        public static create(properties?: HashTableMessage.IPublish): HashTableMessage.Publish;

        /**
         * Encodes the specified Publish message. Does not implicitly {@link HashTableMessage.Publish.verify|verify} messages.
         * @param message Publish message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: HashTableMessage.IPublish, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Publish message, length delimited. Does not implicitly {@link HashTableMessage.Publish.verify|verify} messages.
         * @param message Publish message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: HashTableMessage.IPublish, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a Publish message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Publish
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): HashTableMessage.Publish;

        /**
         * Decodes a Publish message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Publish
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): HashTableMessage.Publish;

        /**
         * Verifies a Publish message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a Publish message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Publish
         */
        public static fromObject(object: { [k: string]: any }): HashTableMessage.Publish;

        /**
         * Creates a plain object from a Publish message. Also converts values to other types if specified.
         * @param message Publish
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: HashTableMessage.Publish, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Publish to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }

    /** Properties of an Unpublish. */
    interface IUnpublish {

        /** Unpublish record */
        record?: (HashTableMessage.IRecord|null);
    }

    /** Represents an Unpublish. */
    class Unpublish implements IUnpublish {

        /**
         * Constructs a new Unpublish.
         * @param [properties] Properties to set
         */
        constructor(properties?: HashTableMessage.IUnpublish);

        /** Unpublish record. */
        public record?: (HashTableMessage.IRecord|null);

        /**
         * Creates a new Unpublish instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Unpublish instance
         */
        public static create(properties?: HashTableMessage.IUnpublish): HashTableMessage.Unpublish;

        /**
         * Encodes the specified Unpublish message. Does not implicitly {@link HashTableMessage.Unpublish.verify|verify} messages.
         * @param message Unpublish message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: HashTableMessage.IUnpublish, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Unpublish message, length delimited. Does not implicitly {@link HashTableMessage.Unpublish.verify|verify} messages.
         * @param message Unpublish message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: HashTableMessage.IUnpublish, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes an Unpublish message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Unpublish
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): HashTableMessage.Unpublish;

        /**
         * Decodes an Unpublish message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Unpublish
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): HashTableMessage.Unpublish;

        /**
         * Verifies an Unpublish message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates an Unpublish message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Unpublish
         */
        public static fromObject(object: { [k: string]: any }): HashTableMessage.Unpublish;

        /**
         * Creates a plain object from an Unpublish message. Also converts values to other types if specified.
         * @param message Unpublish
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: HashTableMessage.Unpublish, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Unpublish to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }

    /** Properties of a GetRequest. */
    interface IGetRequest {

        /** GetRequest requestId */
        requestId?: (number|null);

        /** GetRequest hash */
        hash?: (Uint8Array|null);
    }

    /** Represents a GetRequest. */
    class GetRequest implements IGetRequest {

        /**
         * Constructs a new GetRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: HashTableMessage.IGetRequest);

        /** GetRequest requestId. */
        public requestId: number;

        /** GetRequest hash. */
        public hash: Uint8Array;

        /**
         * Creates a new GetRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns GetRequest instance
         */
        public static create(properties?: HashTableMessage.IGetRequest): HashTableMessage.GetRequest;

        /**
         * Encodes the specified GetRequest message. Does not implicitly {@link HashTableMessage.GetRequest.verify|verify} messages.
         * @param message GetRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: HashTableMessage.IGetRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified GetRequest message, length delimited. Does not implicitly {@link HashTableMessage.GetRequest.verify|verify} messages.
         * @param message GetRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: HashTableMessage.IGetRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a GetRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns GetRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): HashTableMessage.GetRequest;

        /**
         * Decodes a GetRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns GetRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): HashTableMessage.GetRequest;

        /**
         * Verifies a GetRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a GetRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns GetRequest
         */
        public static fromObject(object: { [k: string]: any }): HashTableMessage.GetRequest;

        /**
         * Creates a plain object from a GetRequest message. Also converts values to other types if specified.
         * @param message GetRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: HashTableMessage.GetRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this GetRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }

    /** Properties of a GetResponse. */
    interface IGetResponse {

        /** GetResponse requestId */
        requestId?: (number|null);

        /** GetResponse record */
        record?: (HashTableMessage.IRecord|null);
    }

    /** Represents a GetResponse. */
    class GetResponse implements IGetResponse {

        /**
         * Constructs a new GetResponse.
         * @param [properties] Properties to set
         */
        constructor(properties?: HashTableMessage.IGetResponse);

        /** GetResponse requestId. */
        public requestId: number;

        /** GetResponse record. */
        public record?: (HashTableMessage.IRecord|null);

        /**
         * Creates a new GetResponse instance using the specified properties.
         * @param [properties] Properties to set
         * @returns GetResponse instance
         */
        public static create(properties?: HashTableMessage.IGetResponse): HashTableMessage.GetResponse;

        /**
         * Encodes the specified GetResponse message. Does not implicitly {@link HashTableMessage.GetResponse.verify|verify} messages.
         * @param message GetResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: HashTableMessage.IGetResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified GetResponse message, length delimited. Does not implicitly {@link HashTableMessage.GetResponse.verify|verify} messages.
         * @param message GetResponse message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: HashTableMessage.IGetResponse, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a GetResponse message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns GetResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): HashTableMessage.GetResponse;

        /**
         * Decodes a GetResponse message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns GetResponse
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): HashTableMessage.GetResponse;

        /**
         * Verifies a GetResponse message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a GetResponse message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns GetResponse
         */
        public static fromObject(object: { [k: string]: any }): HashTableMessage.GetResponse;

        /**
         * Creates a plain object from a GetResponse message. Also converts values to other types if specified.
         * @param message GetResponse
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: HashTableMessage.GetResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this GetResponse to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }
}

/** Properties of a ServerConfig. */
export interface IServerConfig {

    /** ServerConfig key */
    key?: (IKey|null);

    /** ServerConfig nameChangeQuota */
    nameChangeQuota?: (number|null);

    /** ServerConfig tokenTtl */
    tokenTtl?: (google.protobuf.IDuration|null);

    /** ServerConfig roles */
    roles?: (string[]|null);
}

/** Represents a ServerConfig. */
export class ServerConfig implements IServerConfig {

    /**
     * Constructs a new ServerConfig.
     * @param [properties] Properties to set
     */
    constructor(properties?: IServerConfig);

    /** ServerConfig key. */
    public key?: (IKey|null);

    /** ServerConfig nameChangeQuota. */
    public nameChangeQuota: number;

    /** ServerConfig tokenTtl. */
    public tokenTtl?: (google.protobuf.IDuration|null);

    /** ServerConfig roles. */
    public roles: string[];

    /**
     * Creates a new ServerConfig instance using the specified properties.
     * @param [properties] Properties to set
     * @returns ServerConfig instance
     */
    public static create(properties?: IServerConfig): ServerConfig;

    /**
     * Encodes the specified ServerConfig message. Does not implicitly {@link ServerConfig.verify|verify} messages.
     * @param message ServerConfig message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IServerConfig, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified ServerConfig message, length delimited. Does not implicitly {@link ServerConfig.verify|verify} messages.
     * @param message ServerConfig message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IServerConfig, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a ServerConfig message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns ServerConfig
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): ServerConfig;

    /**
     * Decodes a ServerConfig message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns ServerConfig
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): ServerConfig;

    /**
     * Verifies a ServerConfig message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a ServerConfig message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns ServerConfig
     */
    public static fromObject(object: { [k: string]: any }): ServerConfig;

    /**
     * Creates a plain object from a ServerConfig message. Also converts values to other types if specified.
     * @param message ServerConfig
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: ServerConfig, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this ServerConfig to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a NickservNick. */
export interface INickservNick {

    /** NickservNick id */
    id?: (number|null);

    /** NickservNick key */
    key?: (Uint8Array|null);

    /** NickservNick nick */
    nick?: (string|null);

    /** NickservNick remainingNameChangeQuota */
    remainingNameChangeQuota?: (number|null);

    /** NickservNick updatedTimestamp */
    updatedTimestamp?: (number|null);

    /** NickservNick createdTimestamp */
    createdTimestamp?: (number|null);

    /** NickservNick roles */
    roles?: (string[]|null);
}

/** Represents a NickservNick. */
export class NickservNick implements INickservNick {

    /**
     * Constructs a new NickservNick.
     * @param [properties] Properties to set
     */
    constructor(properties?: INickservNick);

    /** NickservNick id. */
    public id: number;

    /** NickservNick key. */
    public key: Uint8Array;

    /** NickservNick nick. */
    public nick: string;

    /** NickservNick remainingNameChangeQuota. */
    public remainingNameChangeQuota: number;

    /** NickservNick updatedTimestamp. */
    public updatedTimestamp: number;

    /** NickservNick createdTimestamp. */
    public createdTimestamp: number;

    /** NickservNick roles. */
    public roles: string[];

    /**
     * Creates a new NickservNick instance using the specified properties.
     * @param [properties] Properties to set
     * @returns NickservNick instance
     */
    public static create(properties?: INickservNick): NickservNick;

    /**
     * Encodes the specified NickservNick message. Does not implicitly {@link NickservNick.verify|verify} messages.
     * @param message NickservNick message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: INickservNick, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified NickservNick message, length delimited. Does not implicitly {@link NickservNick.verify|verify} messages.
     * @param message NickservNick message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: INickservNick, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a NickservNick message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns NickservNick
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): NickservNick;

    /**
     * Decodes a NickservNick message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns NickservNick
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): NickservNick;

    /**
     * Verifies a NickservNick message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a NickservNick message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns NickservNick
     */
    public static fromObject(object: { [k: string]: any }): NickservNick;

    /**
     * Creates a plain object from a NickservNick message. Also converts values to other types if specified.
     * @param message NickservNick
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: NickservNick, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this NickservNick to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a NickServToken. */
export interface INickServToken {

    /** NickServToken key */
    key?: (Uint8Array|null);

    /** NickServToken nick */
    nick?: (string|null);

    /** NickServToken validUntil */
    validUntil?: (google.protobuf.ITimestamp|null);

    /** NickServToken signature */
    signature?: (Uint8Array|null);

    /** NickServToken roles */
    roles?: (string[]|null);
}

/** Represents a NickServToken. */
export class NickServToken implements INickServToken {

    /**
     * Constructs a new NickServToken.
     * @param [properties] Properties to set
     */
    constructor(properties?: INickServToken);

    /** NickServToken key. */
    public key: Uint8Array;

    /** NickServToken nick. */
    public nick: string;

    /** NickServToken validUntil. */
    public validUntil?: (google.protobuf.ITimestamp|null);

    /** NickServToken signature. */
    public signature: Uint8Array;

    /** NickServToken roles. */
    public roles: string[];

    /**
     * Creates a new NickServToken instance using the specified properties.
     * @param [properties] Properties to set
     * @returns NickServToken instance
     */
    public static create(properties?: INickServToken): NickServToken;

    /**
     * Encodes the specified NickServToken message. Does not implicitly {@link NickServToken.verify|verify} messages.
     * @param message NickServToken message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: INickServToken, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified NickServToken message, length delimited. Does not implicitly {@link NickServToken.verify|verify} messages.
     * @param message NickServToken message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: INickServToken, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a NickServToken message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns NickServToken
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): NickServToken;

    /**
     * Decodes a NickServToken message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns NickServToken
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): NickServToken;

    /**
     * Verifies a NickServToken message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a NickServToken message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns NickServToken
     */
    public static fromObject(object: { [k: string]: any }): NickServToken;

    /**
     * Creates a plain object from a NickServToken message. Also converts values to other types if specified.
     * @param message NickServToken
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: NickServToken, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this NickServToken to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a NickServRPCCommand. */
export interface INickServRPCCommand {

    /** NickServRPCCommand requestId */
    requestId?: (number|null);

    /** NickServRPCCommand sourcePublicKey */
    sourcePublicKey?: (Uint8Array|null);

    /** NickServRPCCommand create */
    create?: (NickServRPCCommand.ICreate|null);

    /** NickServRPCCommand retrieve */
    retrieve?: (NickServRPCCommand.IRetrieve|null);

    /** NickServRPCCommand update */
    update?: (NickServRPCCommand.IUpdate|null);

    /** NickServRPCCommand delete */
    "delete"?: (NickServRPCCommand.IDelete|null);
}

/** Represents a NickServRPCCommand. */
export class NickServRPCCommand implements INickServRPCCommand {

    /**
     * Constructs a new NickServRPCCommand.
     * @param [properties] Properties to set
     */
    constructor(properties?: INickServRPCCommand);

    /** NickServRPCCommand requestId. */
    public requestId: number;

    /** NickServRPCCommand sourcePublicKey. */
    public sourcePublicKey: Uint8Array;

    /** NickServRPCCommand create. */
    public create?: (NickServRPCCommand.ICreate|null);

    /** NickServRPCCommand retrieve. */
    public retrieve?: (NickServRPCCommand.IRetrieve|null);

    /** NickServRPCCommand update. */
    public update?: (NickServRPCCommand.IUpdate|null);

    /** NickServRPCCommand delete. */
    public delete?: (NickServRPCCommand.IDelete|null);

    /** NickServRPCCommand body. */
    public body?: ("create"|"retrieve"|"update"|"delete");

    /**
     * Creates a new NickServRPCCommand instance using the specified properties.
     * @param [properties] Properties to set
     * @returns NickServRPCCommand instance
     */
    public static create(properties?: INickServRPCCommand): NickServRPCCommand;

    /**
     * Encodes the specified NickServRPCCommand message. Does not implicitly {@link NickServRPCCommand.verify|verify} messages.
     * @param message NickServRPCCommand message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: INickServRPCCommand, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified NickServRPCCommand message, length delimited. Does not implicitly {@link NickServRPCCommand.verify|verify} messages.
     * @param message NickServRPCCommand message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: INickServRPCCommand, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a NickServRPCCommand message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns NickServRPCCommand
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): NickServRPCCommand;

    /**
     * Decodes a NickServRPCCommand message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns NickServRPCCommand
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): NickServRPCCommand;

    /**
     * Verifies a NickServRPCCommand message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a NickServRPCCommand message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns NickServRPCCommand
     */
    public static fromObject(object: { [k: string]: any }): NickServRPCCommand;

    /**
     * Creates a plain object from a NickServRPCCommand message. Also converts values to other types if specified.
     * @param message NickServRPCCommand
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: NickServRPCCommand, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this NickServRPCCommand to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

export namespace NickServRPCCommand {

    /** Properties of a Create. */
    interface ICreate {

        /** Create nick */
        nick?: (string|null);
    }

    /** Represents a Create. */
    class Create implements ICreate {

        /**
         * Constructs a new Create.
         * @param [properties] Properties to set
         */
        constructor(properties?: NickServRPCCommand.ICreate);

        /** Create nick. */
        public nick: string;

        /**
         * Creates a new Create instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Create instance
         */
        public static create(properties?: NickServRPCCommand.ICreate): NickServRPCCommand.Create;

        /**
         * Encodes the specified Create message. Does not implicitly {@link NickServRPCCommand.Create.verify|verify} messages.
         * @param message Create message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: NickServRPCCommand.ICreate, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Create message, length delimited. Does not implicitly {@link NickServRPCCommand.Create.verify|verify} messages.
         * @param message Create message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: NickServRPCCommand.ICreate, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a Create message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Create
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): NickServRPCCommand.Create;

        /**
         * Decodes a Create message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Create
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): NickServRPCCommand.Create;

        /**
         * Verifies a Create message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a Create message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Create
         */
        public static fromObject(object: { [k: string]: any }): NickServRPCCommand.Create;

        /**
         * Creates a plain object from a Create message. Also converts values to other types if specified.
         * @param message Create
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: NickServRPCCommand.Create, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Create to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }

    /** Properties of a Retrieve. */
    interface IRetrieve {
    }

    /** Represents a Retrieve. */
    class Retrieve implements IRetrieve {

        /**
         * Constructs a new Retrieve.
         * @param [properties] Properties to set
         */
        constructor(properties?: NickServRPCCommand.IRetrieve);

        /**
         * Creates a new Retrieve instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Retrieve instance
         */
        public static create(properties?: NickServRPCCommand.IRetrieve): NickServRPCCommand.Retrieve;

        /**
         * Encodes the specified Retrieve message. Does not implicitly {@link NickServRPCCommand.Retrieve.verify|verify} messages.
         * @param message Retrieve message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: NickServRPCCommand.IRetrieve, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Retrieve message, length delimited. Does not implicitly {@link NickServRPCCommand.Retrieve.verify|verify} messages.
         * @param message Retrieve message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: NickServRPCCommand.IRetrieve, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a Retrieve message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Retrieve
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): NickServRPCCommand.Retrieve;

        /**
         * Decodes a Retrieve message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Retrieve
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): NickServRPCCommand.Retrieve;

        /**
         * Verifies a Retrieve message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a Retrieve message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Retrieve
         */
        public static fromObject(object: { [k: string]: any }): NickServRPCCommand.Retrieve;

        /**
         * Creates a plain object from a Retrieve message. Also converts values to other types if specified.
         * @param message Retrieve
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: NickServRPCCommand.Retrieve, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Retrieve to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }

    /** Properties of an Update. */
    interface IUpdate {

        /** Update nick */
        nick?: (NickServRPCCommand.Update.IChangeNick|null);

        /** Update nameChangeQuota */
        nameChangeQuota?: (number|null);

        /** Update roles */
        roles?: (NickServRPCCommand.Update.IRoles|null);
    }

    /** Represents an Update. */
    class Update implements IUpdate {

        /**
         * Constructs a new Update.
         * @param [properties] Properties to set
         */
        constructor(properties?: NickServRPCCommand.IUpdate);

        /** Update nick. */
        public nick?: (NickServRPCCommand.Update.IChangeNick|null);

        /** Update nameChangeQuota. */
        public nameChangeQuota: number;

        /** Update roles. */
        public roles?: (NickServRPCCommand.Update.IRoles|null);

        /** Update param. */
        public param?: ("nick"|"nameChangeQuota"|"roles");

        /**
         * Creates a new Update instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Update instance
         */
        public static create(properties?: NickServRPCCommand.IUpdate): NickServRPCCommand.Update;

        /**
         * Encodes the specified Update message. Does not implicitly {@link NickServRPCCommand.Update.verify|verify} messages.
         * @param message Update message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: NickServRPCCommand.IUpdate, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Update message, length delimited. Does not implicitly {@link NickServRPCCommand.Update.verify|verify} messages.
         * @param message Update message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: NickServRPCCommand.IUpdate, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes an Update message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Update
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): NickServRPCCommand.Update;

        /**
         * Decodes an Update message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Update
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): NickServRPCCommand.Update;

        /**
         * Verifies an Update message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates an Update message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Update
         */
        public static fromObject(object: { [k: string]: any }): NickServRPCCommand.Update;

        /**
         * Creates a plain object from an Update message. Also converts values to other types if specified.
         * @param message Update
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: NickServRPCCommand.Update, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Update to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }

    namespace Update {

        /** Properties of a Roles. */
        interface IRoles {

            /** Roles roles */
            roles?: (string[]|null);
        }

        /** Represents a Roles. */
        class Roles implements IRoles {

            /**
             * Constructs a new Roles.
             * @param [properties] Properties to set
             */
            constructor(properties?: NickServRPCCommand.Update.IRoles);

            /** Roles roles. */
            public roles: string[];

            /**
             * Creates a new Roles instance using the specified properties.
             * @param [properties] Properties to set
             * @returns Roles instance
             */
            public static create(properties?: NickServRPCCommand.Update.IRoles): NickServRPCCommand.Update.Roles;

            /**
             * Encodes the specified Roles message. Does not implicitly {@link NickServRPCCommand.Update.Roles.verify|verify} messages.
             * @param message Roles message or plain object to encode
             * @param [writer] Writer to encode to
             * @returns Writer
             */
            public static encode(message: NickServRPCCommand.Update.IRoles, writer?: $protobuf.Writer): $protobuf.Writer;

            /**
             * Encodes the specified Roles message, length delimited. Does not implicitly {@link NickServRPCCommand.Update.Roles.verify|verify} messages.
             * @param message Roles message or plain object to encode
             * @param [writer] Writer to encode to
             * @returns Writer
             */
            public static encodeDelimited(message: NickServRPCCommand.Update.IRoles, writer?: $protobuf.Writer): $protobuf.Writer;

            /**
             * Decodes a Roles message from the specified reader or buffer.
             * @param reader Reader or buffer to decode from
             * @param [length] Message length if known beforehand
             * @returns Roles
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): NickServRPCCommand.Update.Roles;

            /**
             * Decodes a Roles message from the specified reader or buffer, length delimited.
             * @param reader Reader or buffer to decode from
             * @returns Roles
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): NickServRPCCommand.Update.Roles;

            /**
             * Verifies a Roles message.
             * @param message Plain object to verify
             * @returns `null` if valid, otherwise the reason why it is not
             */
            public static verify(message: { [k: string]: any }): (string|null);

            /**
             * Creates a Roles message from a plain object. Also converts values to their respective internal types.
             * @param object Plain object
             * @returns Roles
             */
            public static fromObject(object: { [k: string]: any }): NickServRPCCommand.Update.Roles;

            /**
             * Creates a plain object from a Roles message. Also converts values to other types if specified.
             * @param message Roles
             * @param [options] Conversion options
             * @returns Plain object
             */
            public static toObject(message: NickServRPCCommand.Update.Roles, options?: $protobuf.IConversionOptions): { [k: string]: any };

            /**
             * Converts this Roles to JSON.
             * @returns JSON object
             */
            public toJSON(): { [k: string]: any };
        }

        /** Properties of a ChangeNick. */
        interface IChangeNick {

            /** ChangeNick oldNick */
            oldNick?: (string|null);

            /** ChangeNick newNick */
            newNick?: (string|null);
        }

        /** Represents a ChangeNick. */
        class ChangeNick implements IChangeNick {

            /**
             * Constructs a new ChangeNick.
             * @param [properties] Properties to set
             */
            constructor(properties?: NickServRPCCommand.Update.IChangeNick);

            /** ChangeNick oldNick. */
            public oldNick: string;

            /** ChangeNick newNick. */
            public newNick: string;

            /**
             * Creates a new ChangeNick instance using the specified properties.
             * @param [properties] Properties to set
             * @returns ChangeNick instance
             */
            public static create(properties?: NickServRPCCommand.Update.IChangeNick): NickServRPCCommand.Update.ChangeNick;

            /**
             * Encodes the specified ChangeNick message. Does not implicitly {@link NickServRPCCommand.Update.ChangeNick.verify|verify} messages.
             * @param message ChangeNick message or plain object to encode
             * @param [writer] Writer to encode to
             * @returns Writer
             */
            public static encode(message: NickServRPCCommand.Update.IChangeNick, writer?: $protobuf.Writer): $protobuf.Writer;

            /**
             * Encodes the specified ChangeNick message, length delimited. Does not implicitly {@link NickServRPCCommand.Update.ChangeNick.verify|verify} messages.
             * @param message ChangeNick message or plain object to encode
             * @param [writer] Writer to encode to
             * @returns Writer
             */
            public static encodeDelimited(message: NickServRPCCommand.Update.IChangeNick, writer?: $protobuf.Writer): $protobuf.Writer;

            /**
             * Decodes a ChangeNick message from the specified reader or buffer.
             * @param reader Reader or buffer to decode from
             * @param [length] Message length if known beforehand
             * @returns ChangeNick
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): NickServRPCCommand.Update.ChangeNick;

            /**
             * Decodes a ChangeNick message from the specified reader or buffer, length delimited.
             * @param reader Reader or buffer to decode from
             * @returns ChangeNick
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): NickServRPCCommand.Update.ChangeNick;

            /**
             * Verifies a ChangeNick message.
             * @param message Plain object to verify
             * @returns `null` if valid, otherwise the reason why it is not
             */
            public static verify(message: { [k: string]: any }): (string|null);

            /**
             * Creates a ChangeNick message from a plain object. Also converts values to their respective internal types.
             * @param object Plain object
             * @returns ChangeNick
             */
            public static fromObject(object: { [k: string]: any }): NickServRPCCommand.Update.ChangeNick;

            /**
             * Creates a plain object from a ChangeNick message. Also converts values to other types if specified.
             * @param message ChangeNick
             * @param [options] Conversion options
             * @returns Plain object
             */
            public static toObject(message: NickServRPCCommand.Update.ChangeNick, options?: $protobuf.IConversionOptions): { [k: string]: any };

            /**
             * Converts this ChangeNick to JSON.
             * @returns JSON object
             */
            public toJSON(): { [k: string]: any };
        }
    }

    /** Properties of a Delete. */
    interface IDelete {
    }

    /** Represents a Delete. */
    class Delete implements IDelete {

        /**
         * Constructs a new Delete.
         * @param [properties] Properties to set
         */
        constructor(properties?: NickServRPCCommand.IDelete);

        /**
         * Creates a new Delete instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Delete instance
         */
        public static create(properties?: NickServRPCCommand.IDelete): NickServRPCCommand.Delete;

        /**
         * Encodes the specified Delete message. Does not implicitly {@link NickServRPCCommand.Delete.verify|verify} messages.
         * @param message Delete message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: NickServRPCCommand.IDelete, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Delete message, length delimited. Does not implicitly {@link NickServRPCCommand.Delete.verify|verify} messages.
         * @param message Delete message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: NickServRPCCommand.IDelete, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a Delete message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Delete
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): NickServRPCCommand.Delete;

        /**
         * Decodes a Delete message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Delete
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): NickServRPCCommand.Delete;

        /**
         * Verifies a Delete message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a Delete message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Delete
         */
        public static fromObject(object: { [k: string]: any }): NickServRPCCommand.Delete;

        /**
         * Creates a plain object from a Delete message. Also converts values to other types if specified.
         * @param message Delete
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: NickServRPCCommand.Delete, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Delete to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }
}

/** Properties of a NickServRPCResponse. */
export interface INickServRPCResponse {

    /** NickServRPCResponse requestId */
    requestId?: (number|null);

    /** NickServRPCResponse error */
    error?: (string|null);

    /** NickServRPCResponse update */
    update?: (NickServRPCResponse.IUpdate|null);

    /** NickServRPCResponse delete */
    "delete"?: (NickServRPCResponse.IDelete|null);

    /** NickServRPCResponse create */
    create?: (INickServToken|null);

    /** NickServRPCResponse retrieve */
    retrieve?: (INickServToken|null);
}

/** Represents a NickServRPCResponse. */
export class NickServRPCResponse implements INickServRPCResponse {

    /**
     * Constructs a new NickServRPCResponse.
     * @param [properties] Properties to set
     */
    constructor(properties?: INickServRPCResponse);

    /** NickServRPCResponse requestId. */
    public requestId: number;

    /** NickServRPCResponse error. */
    public error: string;

    /** NickServRPCResponse update. */
    public update?: (NickServRPCResponse.IUpdate|null);

    /** NickServRPCResponse delete. */
    public delete?: (NickServRPCResponse.IDelete|null);

    /** NickServRPCResponse create. */
    public create?: (INickServToken|null);

    /** NickServRPCResponse retrieve. */
    public retrieve?: (INickServToken|null);

    /** NickServRPCResponse body. */
    public body?: ("error"|"update"|"delete"|"create"|"retrieve");

    /**
     * Creates a new NickServRPCResponse instance using the specified properties.
     * @param [properties] Properties to set
     * @returns NickServRPCResponse instance
     */
    public static create(properties?: INickServRPCResponse): NickServRPCResponse;

    /**
     * Encodes the specified NickServRPCResponse message. Does not implicitly {@link NickServRPCResponse.verify|verify} messages.
     * @param message NickServRPCResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: INickServRPCResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified NickServRPCResponse message, length delimited. Does not implicitly {@link NickServRPCResponse.verify|verify} messages.
     * @param message NickServRPCResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: INickServRPCResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a NickServRPCResponse message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns NickServRPCResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): NickServRPCResponse;

    /**
     * Decodes a NickServRPCResponse message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns NickServRPCResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): NickServRPCResponse;

    /**
     * Verifies a NickServRPCResponse message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a NickServRPCResponse message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns NickServRPCResponse
     */
    public static fromObject(object: { [k: string]: any }): NickServRPCResponse;

    /**
     * Creates a plain object from a NickServRPCResponse message. Also converts values to other types if specified.
     * @param message NickServRPCResponse
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: NickServRPCResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this NickServRPCResponse to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

export namespace NickServRPCResponse {

    /** Properties of an Update. */
    interface IUpdate {
    }

    /** Represents an Update. */
    class Update implements IUpdate {

        /**
         * Constructs a new Update.
         * @param [properties] Properties to set
         */
        constructor(properties?: NickServRPCResponse.IUpdate);

        /**
         * Creates a new Update instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Update instance
         */
        public static create(properties?: NickServRPCResponse.IUpdate): NickServRPCResponse.Update;

        /**
         * Encodes the specified Update message. Does not implicitly {@link NickServRPCResponse.Update.verify|verify} messages.
         * @param message Update message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: NickServRPCResponse.IUpdate, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Update message, length delimited. Does not implicitly {@link NickServRPCResponse.Update.verify|verify} messages.
         * @param message Update message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: NickServRPCResponse.IUpdate, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes an Update message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Update
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): NickServRPCResponse.Update;

        /**
         * Decodes an Update message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Update
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): NickServRPCResponse.Update;

        /**
         * Verifies an Update message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates an Update message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Update
         */
        public static fromObject(object: { [k: string]: any }): NickServRPCResponse.Update;

        /**
         * Creates a plain object from an Update message. Also converts values to other types if specified.
         * @param message Update
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: NickServRPCResponse.Update, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Update to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }

    /** Properties of a Delete. */
    interface IDelete {
    }

    /** Represents a Delete. */
    class Delete implements IDelete {

        /**
         * Constructs a new Delete.
         * @param [properties] Properties to set
         */
        constructor(properties?: NickServRPCResponse.IDelete);

        /**
         * Creates a new Delete instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Delete instance
         */
        public static create(properties?: NickServRPCResponse.IDelete): NickServRPCResponse.Delete;

        /**
         * Encodes the specified Delete message. Does not implicitly {@link NickServRPCResponse.Delete.verify|verify} messages.
         * @param message Delete message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: NickServRPCResponse.IDelete, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Delete message, length delimited. Does not implicitly {@link NickServRPCResponse.Delete.verify|verify} messages.
         * @param message Delete message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: NickServRPCResponse.IDelete, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a Delete message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Delete
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): NickServRPCResponse.Delete;

        /**
         * Decodes a Delete message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Delete
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): NickServRPCResponse.Delete;

        /**
         * Verifies a Delete message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a Delete message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Delete
         */
        public static fromObject(object: { [k: string]: any }): NickServRPCResponse.Delete;

        /**
         * Creates a plain object from a Delete message. Also converts values to other types if specified.
         * @param message Delete
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: NickServRPCResponse.Delete, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Delete to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }
}

/** Properties of a GetDirectoryEventsRequest. */
export interface IGetDirectoryEventsRequest {

    /** GetDirectoryEventsRequest networkKey */
    networkKey?: (Uint8Array|null);
}

/** Represents a GetDirectoryEventsRequest. */
export class GetDirectoryEventsRequest implements IGetDirectoryEventsRequest {

    /**
     * Constructs a new GetDirectoryEventsRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: IGetDirectoryEventsRequest);

    /** GetDirectoryEventsRequest networkKey. */
    public networkKey: Uint8Array;

    /**
     * Creates a new GetDirectoryEventsRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns GetDirectoryEventsRequest instance
     */
    public static create(properties?: IGetDirectoryEventsRequest): GetDirectoryEventsRequest;

    /**
     * Encodes the specified GetDirectoryEventsRequest message. Does not implicitly {@link GetDirectoryEventsRequest.verify|verify} messages.
     * @param message GetDirectoryEventsRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IGetDirectoryEventsRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified GetDirectoryEventsRequest message, length delimited. Does not implicitly {@link GetDirectoryEventsRequest.verify|verify} messages.
     * @param message GetDirectoryEventsRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IGetDirectoryEventsRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a GetDirectoryEventsRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns GetDirectoryEventsRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): GetDirectoryEventsRequest;

    /**
     * Decodes a GetDirectoryEventsRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns GetDirectoryEventsRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): GetDirectoryEventsRequest;

    /**
     * Verifies a GetDirectoryEventsRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a GetDirectoryEventsRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns GetDirectoryEventsRequest
     */
    public static fromObject(object: { [k: string]: any }): GetDirectoryEventsRequest;

    /**
     * Creates a plain object from a GetDirectoryEventsRequest message. Also converts values to other types if specified.
     * @param message GetDirectoryEventsRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: GetDirectoryEventsRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this GetDirectoryEventsRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a TestDirectoryPublishRequest. */
export interface ITestDirectoryPublishRequest {

    /** TestDirectoryPublishRequest networkKey */
    networkKey?: (Uint8Array|null);
}

/** Represents a TestDirectoryPublishRequest. */
export class TestDirectoryPublishRequest implements ITestDirectoryPublishRequest {

    /**
     * Constructs a new TestDirectoryPublishRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: ITestDirectoryPublishRequest);

    /** TestDirectoryPublishRequest networkKey. */
    public networkKey: Uint8Array;

    /**
     * Creates a new TestDirectoryPublishRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns TestDirectoryPublishRequest instance
     */
    public static create(properties?: ITestDirectoryPublishRequest): TestDirectoryPublishRequest;

    /**
     * Encodes the specified TestDirectoryPublishRequest message. Does not implicitly {@link TestDirectoryPublishRequest.verify|verify} messages.
     * @param message TestDirectoryPublishRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: ITestDirectoryPublishRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified TestDirectoryPublishRequest message, length delimited. Does not implicitly {@link TestDirectoryPublishRequest.verify|verify} messages.
     * @param message TestDirectoryPublishRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: ITestDirectoryPublishRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a TestDirectoryPublishRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns TestDirectoryPublishRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): TestDirectoryPublishRequest;

    /**
     * Decodes a TestDirectoryPublishRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns TestDirectoryPublishRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): TestDirectoryPublishRequest;

    /**
     * Verifies a TestDirectoryPublishRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a TestDirectoryPublishRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns TestDirectoryPublishRequest
     */
    public static fromObject(object: { [k: string]: any }): TestDirectoryPublishRequest;

    /**
     * Creates a plain object from a TestDirectoryPublishRequest message. Also converts values to other types if specified.
     * @param message TestDirectoryPublishRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: TestDirectoryPublishRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this TestDirectoryPublishRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a TestDirectoryPublishResponse. */
export interface ITestDirectoryPublishResponse {
}

/** Represents a TestDirectoryPublishResponse. */
export class TestDirectoryPublishResponse implements ITestDirectoryPublishResponse {

    /**
     * Constructs a new TestDirectoryPublishResponse.
     * @param [properties] Properties to set
     */
    constructor(properties?: ITestDirectoryPublishResponse);

    /**
     * Creates a new TestDirectoryPublishResponse instance using the specified properties.
     * @param [properties] Properties to set
     * @returns TestDirectoryPublishResponse instance
     */
    public static create(properties?: ITestDirectoryPublishResponse): TestDirectoryPublishResponse;

    /**
     * Encodes the specified TestDirectoryPublishResponse message. Does not implicitly {@link TestDirectoryPublishResponse.verify|verify} messages.
     * @param message TestDirectoryPublishResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: ITestDirectoryPublishResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified TestDirectoryPublishResponse message, length delimited. Does not implicitly {@link TestDirectoryPublishResponse.verify|verify} messages.
     * @param message TestDirectoryPublishResponse message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: ITestDirectoryPublishResponse, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a TestDirectoryPublishResponse message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns TestDirectoryPublishResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): TestDirectoryPublishResponse;

    /**
     * Decodes a TestDirectoryPublishResponse message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns TestDirectoryPublishResponse
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): TestDirectoryPublishResponse;

    /**
     * Verifies a TestDirectoryPublishResponse message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a TestDirectoryPublishResponse message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns TestDirectoryPublishResponse
     */
    public static fromObject(object: { [k: string]: any }): TestDirectoryPublishResponse;

    /**
     * Creates a plain object from a TestDirectoryPublishResponse message. Also converts values to other types if specified.
     * @param message TestDirectoryPublishResponse
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: TestDirectoryPublishResponse, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this TestDirectoryPublishResponse to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a DirectoryListing. */
export interface IDirectoryListing {

    /** DirectoryListing key */
    key?: (Uint8Array|null);

    /** DirectoryListing mimeType */
    mimeType?: (string|null);

    /** DirectoryListing title */
    title?: (string|null);

    /** DirectoryListing description */
    description?: (string|null);

    /** DirectoryListing tags */
    tags?: (string[]|null);

    /** DirectoryListing extra */
    extra?: (Uint8Array|null);
}

/** Represents a DirectoryListing. */
export class DirectoryListing implements IDirectoryListing {

    /**
     * Constructs a new DirectoryListing.
     * @param [properties] Properties to set
     */
    constructor(properties?: IDirectoryListing);

    /** DirectoryListing key. */
    public key: Uint8Array;

    /** DirectoryListing mimeType. */
    public mimeType: string;

    /** DirectoryListing title. */
    public title: string;

    /** DirectoryListing description. */
    public description: string;

    /** DirectoryListing tags. */
    public tags: string[];

    /** DirectoryListing extra. */
    public extra: Uint8Array;

    /**
     * Creates a new DirectoryListing instance using the specified properties.
     * @param [properties] Properties to set
     * @returns DirectoryListing instance
     */
    public static create(properties?: IDirectoryListing): DirectoryListing;

    /**
     * Encodes the specified DirectoryListing message. Does not implicitly {@link DirectoryListing.verify|verify} messages.
     * @param message DirectoryListing message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IDirectoryListing, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified DirectoryListing message, length delimited. Does not implicitly {@link DirectoryListing.verify|verify} messages.
     * @param message DirectoryListing message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IDirectoryListing, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a DirectoryListing message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns DirectoryListing
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): DirectoryListing;

    /**
     * Decodes a DirectoryListing message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns DirectoryListing
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): DirectoryListing;

    /**
     * Verifies a DirectoryListing message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a DirectoryListing message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns DirectoryListing
     */
    public static fromObject(object: { [k: string]: any }): DirectoryListing;

    /**
     * Creates a plain object from a DirectoryListing message. Also converts values to other types if specified.
     * @param message DirectoryListing
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: DirectoryListing, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this DirectoryListing to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a DirectoryServerEvent. */
export interface IDirectoryServerEvent {

    /** DirectoryServerEvent publish */
    publish?: (DirectoryServerEvent.IPublish|null);

    /** DirectoryServerEvent unpublish */
    unpublish?: (DirectoryServerEvent.IUnpublish|null);

    /** DirectoryServerEvent open */
    open?: (DirectoryServerEvent.IViewerChange|null);

    /** DirectoryServerEvent ping */
    ping?: (DirectoryServerEvent.IPing|null);
}

/** Represents a DirectoryServerEvent. */
export class DirectoryServerEvent implements IDirectoryServerEvent {

    /**
     * Constructs a new DirectoryServerEvent.
     * @param [properties] Properties to set
     */
    constructor(properties?: IDirectoryServerEvent);

    /** DirectoryServerEvent publish. */
    public publish?: (DirectoryServerEvent.IPublish|null);

    /** DirectoryServerEvent unpublish. */
    public unpublish?: (DirectoryServerEvent.IUnpublish|null);

    /** DirectoryServerEvent open. */
    public open?: (DirectoryServerEvent.IViewerChange|null);

    /** DirectoryServerEvent ping. */
    public ping?: (DirectoryServerEvent.IPing|null);

    /** DirectoryServerEvent body. */
    public body?: ("publish"|"unpublish"|"open"|"ping");

    /**
     * Creates a new DirectoryServerEvent instance using the specified properties.
     * @param [properties] Properties to set
     * @returns DirectoryServerEvent instance
     */
    public static create(properties?: IDirectoryServerEvent): DirectoryServerEvent;

    /**
     * Encodes the specified DirectoryServerEvent message. Does not implicitly {@link DirectoryServerEvent.verify|verify} messages.
     * @param message DirectoryServerEvent message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IDirectoryServerEvent, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified DirectoryServerEvent message, length delimited. Does not implicitly {@link DirectoryServerEvent.verify|verify} messages.
     * @param message DirectoryServerEvent message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IDirectoryServerEvent, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a DirectoryServerEvent message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns DirectoryServerEvent
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): DirectoryServerEvent;

    /**
     * Decodes a DirectoryServerEvent message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns DirectoryServerEvent
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): DirectoryServerEvent;

    /**
     * Verifies a DirectoryServerEvent message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a DirectoryServerEvent message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns DirectoryServerEvent
     */
    public static fromObject(object: { [k: string]: any }): DirectoryServerEvent;

    /**
     * Creates a plain object from a DirectoryServerEvent message. Also converts values to other types if specified.
     * @param message DirectoryServerEvent
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: DirectoryServerEvent, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this DirectoryServerEvent to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

export namespace DirectoryServerEvent {

    /** Properties of a Publish. */
    interface IPublish {

        /** Publish listing */
        listing?: (IDirectoryListing|null);
    }

    /** Represents a Publish. */
    class Publish implements IPublish {

        /**
         * Constructs a new Publish.
         * @param [properties] Properties to set
         */
        constructor(properties?: DirectoryServerEvent.IPublish);

        /** Publish listing. */
        public listing?: (IDirectoryListing|null);

        /**
         * Creates a new Publish instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Publish instance
         */
        public static create(properties?: DirectoryServerEvent.IPublish): DirectoryServerEvent.Publish;

        /**
         * Encodes the specified Publish message. Does not implicitly {@link DirectoryServerEvent.Publish.verify|verify} messages.
         * @param message Publish message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: DirectoryServerEvent.IPublish, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Publish message, length delimited. Does not implicitly {@link DirectoryServerEvent.Publish.verify|verify} messages.
         * @param message Publish message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: DirectoryServerEvent.IPublish, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a Publish message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Publish
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): DirectoryServerEvent.Publish;

        /**
         * Decodes a Publish message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Publish
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): DirectoryServerEvent.Publish;

        /**
         * Verifies a Publish message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a Publish message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Publish
         */
        public static fromObject(object: { [k: string]: any }): DirectoryServerEvent.Publish;

        /**
         * Creates a plain object from a Publish message. Also converts values to other types if specified.
         * @param message Publish
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: DirectoryServerEvent.Publish, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Publish to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }

    /** Properties of an Unpublish. */
    interface IUnpublish {

        /** Unpublish key */
        key?: (Uint8Array|null);
    }

    /** Represents an Unpublish. */
    class Unpublish implements IUnpublish {

        /**
         * Constructs a new Unpublish.
         * @param [properties] Properties to set
         */
        constructor(properties?: DirectoryServerEvent.IUnpublish);

        /** Unpublish key. */
        public key: Uint8Array;

        /**
         * Creates a new Unpublish instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Unpublish instance
         */
        public static create(properties?: DirectoryServerEvent.IUnpublish): DirectoryServerEvent.Unpublish;

        /**
         * Encodes the specified Unpublish message. Does not implicitly {@link DirectoryServerEvent.Unpublish.verify|verify} messages.
         * @param message Unpublish message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: DirectoryServerEvent.IUnpublish, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Unpublish message, length delimited. Does not implicitly {@link DirectoryServerEvent.Unpublish.verify|verify} messages.
         * @param message Unpublish message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: DirectoryServerEvent.IUnpublish, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes an Unpublish message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Unpublish
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): DirectoryServerEvent.Unpublish;

        /**
         * Decodes an Unpublish message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Unpublish
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): DirectoryServerEvent.Unpublish;

        /**
         * Verifies an Unpublish message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates an Unpublish message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Unpublish
         */
        public static fromObject(object: { [k: string]: any }): DirectoryServerEvent.Unpublish;

        /**
         * Creates a plain object from an Unpublish message. Also converts values to other types if specified.
         * @param message Unpublish
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: DirectoryServerEvent.Unpublish, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Unpublish to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }

    /** Properties of a ViewerChange. */
    interface IViewerChange {

        /** ViewerChange key */
        key?: (Uint8Array|null);

        /** ViewerChange count */
        count?: (number|null);
    }

    /** Represents a ViewerChange. */
    class ViewerChange implements IViewerChange {

        /**
         * Constructs a new ViewerChange.
         * @param [properties] Properties to set
         */
        constructor(properties?: DirectoryServerEvent.IViewerChange);

        /** ViewerChange key. */
        public key: Uint8Array;

        /** ViewerChange count. */
        public count: number;

        /**
         * Creates a new ViewerChange instance using the specified properties.
         * @param [properties] Properties to set
         * @returns ViewerChange instance
         */
        public static create(properties?: DirectoryServerEvent.IViewerChange): DirectoryServerEvent.ViewerChange;

        /**
         * Encodes the specified ViewerChange message. Does not implicitly {@link DirectoryServerEvent.ViewerChange.verify|verify} messages.
         * @param message ViewerChange message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: DirectoryServerEvent.IViewerChange, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified ViewerChange message, length delimited. Does not implicitly {@link DirectoryServerEvent.ViewerChange.verify|verify} messages.
         * @param message ViewerChange message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: DirectoryServerEvent.IViewerChange, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a ViewerChange message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns ViewerChange
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): DirectoryServerEvent.ViewerChange;

        /**
         * Decodes a ViewerChange message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns ViewerChange
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): DirectoryServerEvent.ViewerChange;

        /**
         * Verifies a ViewerChange message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a ViewerChange message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns ViewerChange
         */
        public static fromObject(object: { [k: string]: any }): DirectoryServerEvent.ViewerChange;

        /**
         * Creates a plain object from a ViewerChange message. Also converts values to other types if specified.
         * @param message ViewerChange
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: DirectoryServerEvent.ViewerChange, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this ViewerChange to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }

    /** Properties of a Ping. */
    interface IPing {

        /** Ping time */
        time?: (number|null);
    }

    /** Represents a Ping. */
    class Ping implements IPing {

        /**
         * Constructs a new Ping.
         * @param [properties] Properties to set
         */
        constructor(properties?: DirectoryServerEvent.IPing);

        /** Ping time. */
        public time: number;

        /**
         * Creates a new Ping instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Ping instance
         */
        public static create(properties?: DirectoryServerEvent.IPing): DirectoryServerEvent.Ping;

        /**
         * Encodes the specified Ping message. Does not implicitly {@link DirectoryServerEvent.Ping.verify|verify} messages.
         * @param message Ping message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: DirectoryServerEvent.IPing, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Ping message, length delimited. Does not implicitly {@link DirectoryServerEvent.Ping.verify|verify} messages.
         * @param message Ping message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: DirectoryServerEvent.IPing, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a Ping message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Ping
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): DirectoryServerEvent.Ping;

        /**
         * Decodes a Ping message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Ping
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): DirectoryServerEvent.Ping;

        /**
         * Verifies a Ping message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a Ping message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Ping
         */
        public static fromObject(object: { [k: string]: any }): DirectoryServerEvent.Ping;

        /**
         * Creates a plain object from a Ping message. Also converts values to other types if specified.
         * @param message Ping
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: DirectoryServerEvent.Ping, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Ping to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }
}

/** Properties of a CallDirectoryServerRequest. */
export interface ICallDirectoryServerRequest {

    /** CallDirectoryServerRequest networkKey */
    networkKey?: (Uint8Array|null);

    /** CallDirectoryServerRequest listing */
    listing?: (CallDirectoryServerRequest.IRemoveListing|null);
}

/** Represents a CallDirectoryServerRequest. */
export class CallDirectoryServerRequest implements ICallDirectoryServerRequest {

    /**
     * Constructs a new CallDirectoryServerRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: ICallDirectoryServerRequest);

    /** CallDirectoryServerRequest networkKey. */
    public networkKey: Uint8Array;

    /** CallDirectoryServerRequest listing. */
    public listing?: (CallDirectoryServerRequest.IRemoveListing|null);

    /** CallDirectoryServerRequest body. */
    public body?: "listing";

    /**
     * Creates a new CallDirectoryServerRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns CallDirectoryServerRequest instance
     */
    public static create(properties?: ICallDirectoryServerRequest): CallDirectoryServerRequest;

    /**
     * Encodes the specified CallDirectoryServerRequest message. Does not implicitly {@link CallDirectoryServerRequest.verify|verify} messages.
     * @param message CallDirectoryServerRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: ICallDirectoryServerRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified CallDirectoryServerRequest message, length delimited. Does not implicitly {@link CallDirectoryServerRequest.verify|verify} messages.
     * @param message CallDirectoryServerRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: ICallDirectoryServerRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a CallDirectoryServerRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns CallDirectoryServerRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): CallDirectoryServerRequest;

    /**
     * Decodes a CallDirectoryServerRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns CallDirectoryServerRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): CallDirectoryServerRequest;

    /**
     * Verifies a CallDirectoryServerRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a CallDirectoryServerRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns CallDirectoryServerRequest
     */
    public static fromObject(object: { [k: string]: any }): CallDirectoryServerRequest;

    /**
     * Creates a plain object from a CallDirectoryServerRequest message. Also converts values to other types if specified.
     * @param message CallDirectoryServerRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: CallDirectoryServerRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this CallDirectoryServerRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

export namespace CallDirectoryServerRequest {

    /** Properties of a RemoveListing. */
    interface IRemoveListing {

        /** RemoveListing key */
        key?: (Uint8Array|null);
    }

    /** Represents a RemoveListing. */
    class RemoveListing implements IRemoveListing {

        /**
         * Constructs a new RemoveListing.
         * @param [properties] Properties to set
         */
        constructor(properties?: CallDirectoryServerRequest.IRemoveListing);

        /** RemoveListing key. */
        public key: Uint8Array;

        /**
         * Creates a new RemoveListing instance using the specified properties.
         * @param [properties] Properties to set
         * @returns RemoveListing instance
         */
        public static create(properties?: CallDirectoryServerRequest.IRemoveListing): CallDirectoryServerRequest.RemoveListing;

        /**
         * Encodes the specified RemoveListing message. Does not implicitly {@link CallDirectoryServerRequest.RemoveListing.verify|verify} messages.
         * @param message RemoveListing message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: CallDirectoryServerRequest.IRemoveListing, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified RemoveListing message, length delimited. Does not implicitly {@link CallDirectoryServerRequest.RemoveListing.verify|verify} messages.
         * @param message RemoveListing message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: CallDirectoryServerRequest.IRemoveListing, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a RemoveListing message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns RemoveListing
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): CallDirectoryServerRequest.RemoveListing;

        /**
         * Decodes a RemoveListing message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns RemoveListing
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): CallDirectoryServerRequest.RemoveListing;

        /**
         * Verifies a RemoveListing message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a RemoveListing message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns RemoveListing
         */
        public static fromObject(object: { [k: string]: any }): CallDirectoryServerRequest.RemoveListing;

        /**
         * Creates a plain object from a RemoveListing message. Also converts values to other types if specified.
         * @param message RemoveListing
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: CallDirectoryServerRequest.RemoveListing, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this RemoveListing to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }
}

/** Properties of an OpenDirectoryClientRequest. */
export interface IOpenDirectoryClientRequest {

    /** OpenDirectoryClientRequest networkKey */
    networkKey?: (Uint8Array|null);

    /** OpenDirectoryClientRequest serverKey */
    serverKey?: (Uint8Array|null);
}

/** Represents an OpenDirectoryClientRequest. */
export class OpenDirectoryClientRequest implements IOpenDirectoryClientRequest {

    /**
     * Constructs a new OpenDirectoryClientRequest.
     * @param [properties] Properties to set
     */
    constructor(properties?: IOpenDirectoryClientRequest);

    /** OpenDirectoryClientRequest networkKey. */
    public networkKey: Uint8Array;

    /** OpenDirectoryClientRequest serverKey. */
    public serverKey: Uint8Array;

    /**
     * Creates a new OpenDirectoryClientRequest instance using the specified properties.
     * @param [properties] Properties to set
     * @returns OpenDirectoryClientRequest instance
     */
    public static create(properties?: IOpenDirectoryClientRequest): OpenDirectoryClientRequest;

    /**
     * Encodes the specified OpenDirectoryClientRequest message. Does not implicitly {@link OpenDirectoryClientRequest.verify|verify} messages.
     * @param message OpenDirectoryClientRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IOpenDirectoryClientRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified OpenDirectoryClientRequest message, length delimited. Does not implicitly {@link OpenDirectoryClientRequest.verify|verify} messages.
     * @param message OpenDirectoryClientRequest message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IOpenDirectoryClientRequest, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes an OpenDirectoryClientRequest message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns OpenDirectoryClientRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): OpenDirectoryClientRequest;

    /**
     * Decodes an OpenDirectoryClientRequest message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns OpenDirectoryClientRequest
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): OpenDirectoryClientRequest;

    /**
     * Verifies an OpenDirectoryClientRequest message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates an OpenDirectoryClientRequest message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns OpenDirectoryClientRequest
     */
    public static fromObject(object: { [k: string]: any }): OpenDirectoryClientRequest;

    /**
     * Creates a plain object from an OpenDirectoryClientRequest message. Also converts values to other types if specified.
     * @param message OpenDirectoryClientRequest
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: OpenDirectoryClientRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this OpenDirectoryClientRequest to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

/** Properties of a DirectoryClientEvent. */
export interface IDirectoryClientEvent {

    /** DirectoryClientEvent publish */
    publish?: (DirectoryClientEvent.IPublish|null);

    /** DirectoryClientEvent unpublish */
    unpublish?: (DirectoryClientEvent.IUnpublish|null);

    /** DirectoryClientEvent join */
    join?: (DirectoryClientEvent.IJoin|null);

    /** DirectoryClientEvent part */
    part?: (DirectoryClientEvent.IPart|null);

    /** DirectoryClientEvent ping */
    ping?: (DirectoryClientEvent.IPing|null);
}

/** Represents a DirectoryClientEvent. */
export class DirectoryClientEvent implements IDirectoryClientEvent {

    /**
     * Constructs a new DirectoryClientEvent.
     * @param [properties] Properties to set
     */
    constructor(properties?: IDirectoryClientEvent);

    /** DirectoryClientEvent publish. */
    public publish?: (DirectoryClientEvent.IPublish|null);

    /** DirectoryClientEvent unpublish. */
    public unpublish?: (DirectoryClientEvent.IUnpublish|null);

    /** DirectoryClientEvent join. */
    public join?: (DirectoryClientEvent.IJoin|null);

    /** DirectoryClientEvent part. */
    public part?: (DirectoryClientEvent.IPart|null);

    /** DirectoryClientEvent ping. */
    public ping?: (DirectoryClientEvent.IPing|null);

    /** DirectoryClientEvent body. */
    public body?: ("publish"|"unpublish"|"join"|"part"|"ping");

    /**
     * Creates a new DirectoryClientEvent instance using the specified properties.
     * @param [properties] Properties to set
     * @returns DirectoryClientEvent instance
     */
    public static create(properties?: IDirectoryClientEvent): DirectoryClientEvent;

    /**
     * Encodes the specified DirectoryClientEvent message. Does not implicitly {@link DirectoryClientEvent.verify|verify} messages.
     * @param message DirectoryClientEvent message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IDirectoryClientEvent, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified DirectoryClientEvent message, length delimited. Does not implicitly {@link DirectoryClientEvent.verify|verify} messages.
     * @param message DirectoryClientEvent message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IDirectoryClientEvent, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a DirectoryClientEvent message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns DirectoryClientEvent
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): DirectoryClientEvent;

    /**
     * Decodes a DirectoryClientEvent message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns DirectoryClientEvent
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): DirectoryClientEvent;

    /**
     * Verifies a DirectoryClientEvent message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a DirectoryClientEvent message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns DirectoryClientEvent
     */
    public static fromObject(object: { [k: string]: any }): DirectoryClientEvent;

    /**
     * Creates a plain object from a DirectoryClientEvent message. Also converts values to other types if specified.
     * @param message DirectoryClientEvent
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: DirectoryClientEvent, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this DirectoryClientEvent to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

export namespace DirectoryClientEvent {

    /** Properties of a Publish. */
    interface IPublish {

        /** Publish listing */
        listing?: (IDirectoryListing|null);

        /** Publish signature */
        signature?: (Uint8Array|null);
    }

    /** Represents a Publish. */
    class Publish implements IPublish {

        /**
         * Constructs a new Publish.
         * @param [properties] Properties to set
         */
        constructor(properties?: DirectoryClientEvent.IPublish);

        /** Publish listing. */
        public listing?: (IDirectoryListing|null);

        /** Publish signature. */
        public signature: Uint8Array;

        /**
         * Creates a new Publish instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Publish instance
         */
        public static create(properties?: DirectoryClientEvent.IPublish): DirectoryClientEvent.Publish;

        /**
         * Encodes the specified Publish message. Does not implicitly {@link DirectoryClientEvent.Publish.verify|verify} messages.
         * @param message Publish message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: DirectoryClientEvent.IPublish, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Publish message, length delimited. Does not implicitly {@link DirectoryClientEvent.Publish.verify|verify} messages.
         * @param message Publish message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: DirectoryClientEvent.IPublish, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a Publish message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Publish
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): DirectoryClientEvent.Publish;

        /**
         * Decodes a Publish message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Publish
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): DirectoryClientEvent.Publish;

        /**
         * Verifies a Publish message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a Publish message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Publish
         */
        public static fromObject(object: { [k: string]: any }): DirectoryClientEvent.Publish;

        /**
         * Creates a plain object from a Publish message. Also converts values to other types if specified.
         * @param message Publish
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: DirectoryClientEvent.Publish, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Publish to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }

    /** Properties of an Unpublish. */
    interface IUnpublish {

        /** Unpublish key */
        key?: (Uint8Array|null);
    }

    /** Represents an Unpublish. */
    class Unpublish implements IUnpublish {

        /**
         * Constructs a new Unpublish.
         * @param [properties] Properties to set
         */
        constructor(properties?: DirectoryClientEvent.IUnpublish);

        /** Unpublish key. */
        public key: Uint8Array;

        /**
         * Creates a new Unpublish instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Unpublish instance
         */
        public static create(properties?: DirectoryClientEvent.IUnpublish): DirectoryClientEvent.Unpublish;

        /**
         * Encodes the specified Unpublish message. Does not implicitly {@link DirectoryClientEvent.Unpublish.verify|verify} messages.
         * @param message Unpublish message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: DirectoryClientEvent.IUnpublish, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Unpublish message, length delimited. Does not implicitly {@link DirectoryClientEvent.Unpublish.verify|verify} messages.
         * @param message Unpublish message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: DirectoryClientEvent.IUnpublish, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes an Unpublish message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Unpublish
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): DirectoryClientEvent.Unpublish;

        /**
         * Decodes an Unpublish message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Unpublish
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): DirectoryClientEvent.Unpublish;

        /**
         * Verifies an Unpublish message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates an Unpublish message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Unpublish
         */
        public static fromObject(object: { [k: string]: any }): DirectoryClientEvent.Unpublish;

        /**
         * Creates a plain object from an Unpublish message. Also converts values to other types if specified.
         * @param message Unpublish
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: DirectoryClientEvent.Unpublish, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Unpublish to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }

    /** Properties of a Join. */
    interface IJoin {

        /** Join key */
        key?: (Uint8Array|null);
    }

    /** Represents a Join. */
    class Join implements IJoin {

        /**
         * Constructs a new Join.
         * @param [properties] Properties to set
         */
        constructor(properties?: DirectoryClientEvent.IJoin);

        /** Join key. */
        public key: Uint8Array;

        /**
         * Creates a new Join instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Join instance
         */
        public static create(properties?: DirectoryClientEvent.IJoin): DirectoryClientEvent.Join;

        /**
         * Encodes the specified Join message. Does not implicitly {@link DirectoryClientEvent.Join.verify|verify} messages.
         * @param message Join message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: DirectoryClientEvent.IJoin, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Join message, length delimited. Does not implicitly {@link DirectoryClientEvent.Join.verify|verify} messages.
         * @param message Join message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: DirectoryClientEvent.IJoin, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a Join message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Join
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): DirectoryClientEvent.Join;

        /**
         * Decodes a Join message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Join
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): DirectoryClientEvent.Join;

        /**
         * Verifies a Join message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a Join message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Join
         */
        public static fromObject(object: { [k: string]: any }): DirectoryClientEvent.Join;

        /**
         * Creates a plain object from a Join message. Also converts values to other types if specified.
         * @param message Join
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: DirectoryClientEvent.Join, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Join to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }

    /** Properties of a Part. */
    interface IPart {

        /** Part key */
        key?: (Uint8Array|null);
    }

    /** Represents a Part. */
    class Part implements IPart {

        /**
         * Constructs a new Part.
         * @param [properties] Properties to set
         */
        constructor(properties?: DirectoryClientEvent.IPart);

        /** Part key. */
        public key: Uint8Array;

        /**
         * Creates a new Part instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Part instance
         */
        public static create(properties?: DirectoryClientEvent.IPart): DirectoryClientEvent.Part;

        /**
         * Encodes the specified Part message. Does not implicitly {@link DirectoryClientEvent.Part.verify|verify} messages.
         * @param message Part message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: DirectoryClientEvent.IPart, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Part message, length delimited. Does not implicitly {@link DirectoryClientEvent.Part.verify|verify} messages.
         * @param message Part message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: DirectoryClientEvent.IPart, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a Part message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Part
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): DirectoryClientEvent.Part;

        /**
         * Decodes a Part message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Part
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): DirectoryClientEvent.Part;

        /**
         * Verifies a Part message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a Part message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Part
         */
        public static fromObject(object: { [k: string]: any }): DirectoryClientEvent.Part;

        /**
         * Creates a plain object from a Part message. Also converts values to other types if specified.
         * @param message Part
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: DirectoryClientEvent.Part, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Part to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }

    /** Properties of a Ping. */
    interface IPing {

        /** Ping time */
        time?: (number|null);
    }

    /** Represents a Ping. */
    class Ping implements IPing {

        /**
         * Constructs a new Ping.
         * @param [properties] Properties to set
         */
        constructor(properties?: DirectoryClientEvent.IPing);

        /** Ping time. */
        public time: number;

        /**
         * Creates a new Ping instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Ping instance
         */
        public static create(properties?: DirectoryClientEvent.IPing): DirectoryClientEvent.Ping;

        /**
         * Encodes the specified Ping message. Does not implicitly {@link DirectoryClientEvent.Ping.verify|verify} messages.
         * @param message Ping message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: DirectoryClientEvent.IPing, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Ping message, length delimited. Does not implicitly {@link DirectoryClientEvent.Ping.verify|verify} messages.
         * @param message Ping message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: DirectoryClientEvent.IPing, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a Ping message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Ping
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): DirectoryClientEvent.Ping;

        /**
         * Decodes a Ping message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Ping
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): DirectoryClientEvent.Ping;

        /**
         * Verifies a Ping message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a Ping message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Ping
         */
        public static fromObject(object: { [k: string]: any }): DirectoryClientEvent.Ping;

        /**
         * Creates a plain object from a Ping message. Also converts values to other types if specified.
         * @param message Ping
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: DirectoryClientEvent.Ping, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Ping to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }
}

/** Properties of a PubSubEvent. */
export interface IPubSubEvent {

    /** PubSubEvent message */
    message?: (PubSubEvent.IMessage|null);

    /** PubSubEvent close */
    close?: (PubSubEvent.IClose|null);

    /** PubSubEvent padding */
    padding?: (PubSubEvent.IPadding|null);
}

/** Represents a PubSubEvent. */
export class PubSubEvent implements IPubSubEvent {

    /**
     * Constructs a new PubSubEvent.
     * @param [properties] Properties to set
     */
    constructor(properties?: IPubSubEvent);

    /** PubSubEvent message. */
    public message?: (PubSubEvent.IMessage|null);

    /** PubSubEvent close. */
    public close?: (PubSubEvent.IClose|null);

    /** PubSubEvent padding. */
    public padding?: (PubSubEvent.IPadding|null);

    /** PubSubEvent body. */
    public body?: ("message"|"close"|"padding");

    /**
     * Creates a new PubSubEvent instance using the specified properties.
     * @param [properties] Properties to set
     * @returns PubSubEvent instance
     */
    public static create(properties?: IPubSubEvent): PubSubEvent;

    /**
     * Encodes the specified PubSubEvent message. Does not implicitly {@link PubSubEvent.verify|verify} messages.
     * @param message PubSubEvent message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encode(message: IPubSubEvent, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Encodes the specified PubSubEvent message, length delimited. Does not implicitly {@link PubSubEvent.verify|verify} messages.
     * @param message PubSubEvent message or plain object to encode
     * @param [writer] Writer to encode to
     * @returns Writer
     */
    public static encodeDelimited(message: IPubSubEvent, writer?: $protobuf.Writer): $protobuf.Writer;

    /**
     * Decodes a PubSubEvent message from the specified reader or buffer.
     * @param reader Reader or buffer to decode from
     * @param [length] Message length if known beforehand
     * @returns PubSubEvent
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): PubSubEvent;

    /**
     * Decodes a PubSubEvent message from the specified reader or buffer, length delimited.
     * @param reader Reader or buffer to decode from
     * @returns PubSubEvent
     * @throws {Error} If the payload is not a reader or valid buffer
     * @throws {$protobuf.util.ProtocolError} If required fields are missing
     */
    public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): PubSubEvent;

    /**
     * Verifies a PubSubEvent message.
     * @param message Plain object to verify
     * @returns `null` if valid, otherwise the reason why it is not
     */
    public static verify(message: { [k: string]: any }): (string|null);

    /**
     * Creates a PubSubEvent message from a plain object. Also converts values to their respective internal types.
     * @param object Plain object
     * @returns PubSubEvent
     */
    public static fromObject(object: { [k: string]: any }): PubSubEvent;

    /**
     * Creates a plain object from a PubSubEvent message. Also converts values to other types if specified.
     * @param message PubSubEvent
     * @param [options] Conversion options
     * @returns Plain object
     */
    public static toObject(message: PubSubEvent, options?: $protobuf.IConversionOptions): { [k: string]: any };

    /**
     * Converts this PubSubEvent to JSON.
     * @returns JSON object
     */
    public toJSON(): { [k: string]: any };
}

export namespace PubSubEvent {

    /** Properties of a Message. */
    interface IMessage {

        /** Message time */
        time?: (number|null);

        /** Message key */
        key?: (string|null);

        /** Message body */
        body?: (Uint8Array|null);
    }

    /** Represents a Message. */
    class Message implements IMessage {

        /**
         * Constructs a new Message.
         * @param [properties] Properties to set
         */
        constructor(properties?: PubSubEvent.IMessage);

        /** Message time. */
        public time: number;

        /** Message key. */
        public key: string;

        /** Message body. */
        public body: Uint8Array;

        /**
         * Creates a new Message instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Message instance
         */
        public static create(properties?: PubSubEvent.IMessage): PubSubEvent.Message;

        /**
         * Encodes the specified Message message. Does not implicitly {@link PubSubEvent.Message.verify|verify} messages.
         * @param message Message message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: PubSubEvent.IMessage, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Message message, length delimited. Does not implicitly {@link PubSubEvent.Message.verify|verify} messages.
         * @param message Message message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: PubSubEvent.IMessage, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a Message message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Message
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): PubSubEvent.Message;

        /**
         * Decodes a Message message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Message
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): PubSubEvent.Message;

        /**
         * Verifies a Message message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a Message message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Message
         */
        public static fromObject(object: { [k: string]: any }): PubSubEvent.Message;

        /**
         * Creates a plain object from a Message message. Also converts values to other types if specified.
         * @param message Message
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: PubSubEvent.Message, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Message to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }

    /** Properties of a Close. */
    interface IClose {
    }

    /** Represents a Close. */
    class Close implements IClose {

        /**
         * Constructs a new Close.
         * @param [properties] Properties to set
         */
        constructor(properties?: PubSubEvent.IClose);

        /**
         * Creates a new Close instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Close instance
         */
        public static create(properties?: PubSubEvent.IClose): PubSubEvent.Close;

        /**
         * Encodes the specified Close message. Does not implicitly {@link PubSubEvent.Close.verify|verify} messages.
         * @param message Close message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: PubSubEvent.IClose, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Close message, length delimited. Does not implicitly {@link PubSubEvent.Close.verify|verify} messages.
         * @param message Close message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: PubSubEvent.IClose, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a Close message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Close
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): PubSubEvent.Close;

        /**
         * Decodes a Close message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Close
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): PubSubEvent.Close;

        /**
         * Verifies a Close message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a Close message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Close
         */
        public static fromObject(object: { [k: string]: any }): PubSubEvent.Close;

        /**
         * Creates a plain object from a Close message. Also converts values to other types if specified.
         * @param message Close
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: PubSubEvent.Close, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Close to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }

    /** Properties of a Padding. */
    interface IPadding {

        /** Padding body */
        body?: (Uint8Array|null);
    }

    /** Represents a Padding. */
    class Padding implements IPadding {

        /**
         * Constructs a new Padding.
         * @param [properties] Properties to set
         */
        constructor(properties?: PubSubEvent.IPadding);

        /** Padding body. */
        public body: Uint8Array;

        /**
         * Creates a new Padding instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Padding instance
         */
        public static create(properties?: PubSubEvent.IPadding): PubSubEvent.Padding;

        /**
         * Encodes the specified Padding message. Does not implicitly {@link PubSubEvent.Padding.verify|verify} messages.
         * @param message Padding message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: PubSubEvent.IPadding, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Padding message, length delimited. Does not implicitly {@link PubSubEvent.Padding.verify|verify} messages.
         * @param message Padding message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: PubSubEvent.IPadding, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a Padding message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Padding
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): PubSubEvent.Padding;

        /**
         * Decodes a Padding message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Padding
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): PubSubEvent.Padding;

        /**
         * Verifies a Padding message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a Padding message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Padding
         */
        public static fromObject(object: { [k: string]: any }): PubSubEvent.Padding;

        /**
         * Creates a plain object from a Padding message. Also converts values to other types if specified.
         * @param message Padding
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: PubSubEvent.Padding, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Padding to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };
    }
}

/** KDFType enum. */
export enum KDFType {
    KDF_TYPE_UNDEFINED = 0,
    KDF_TYPE_PBKDF2_SHA256 = 1
}

/** KeyType enum. */
export enum KeyType {
    KEY_TYPE_UNDEFINED = 0,
    KEY_TYPE_ED25519 = 1,
    KEY_TYPE_X25519 = 2
}

/** KeyUsage enum. */
export enum KeyUsage {
    KEY_USAGE_UNDEFINED = 0,
    KEY_USAGE_PEER = 1,
    KEY_USAGE_BOOTSTRAP = 2,
    KEY_USAGE_SIGN = 4,
    KEY_USAGE_BROKER = 8,
    KEY_USAGE_ENCIPHERMENT = 16
}

/** MetricsFormat enum. */
export enum MetricsFormat {
    METRICS_FORMAT_TEXT = 0,
    METRICS_FORMAT_PROTO_DELIM = 1,
    METRICS_FORMAT_PROTO_TEXT = 2,
    METRICS_FORMAT_PROTO_COMPACT = 3,
    METRICS_FORMAT_OPEN_METRICS = 4
}

/** SwarmEventType enum. */
export enum SwarmEventType {
    CREATE_SWARM = 0,
    UPDATE_SWARM = 1,
    DELETE_SWARM = 2
}

/** WRTCSDPType enum. */
export enum WRTCSDPType {
    OFFER = 0,
    ANSWER = 1
}

/** Namespace google. */
export namespace google {

    /** Namespace protobuf. */
    namespace protobuf {

        /** Properties of a Timestamp. */
        interface ITimestamp {

            /** Timestamp seconds */
            seconds?: (number|null);

            /** Timestamp nanos */
            nanos?: (number|null);
        }

        /** Represents a Timestamp. */
        class Timestamp implements ITimestamp {

            /**
             * Constructs a new Timestamp.
             * @param [properties] Properties to set
             */
            constructor(properties?: google.protobuf.ITimestamp);

            /** Timestamp seconds. */
            public seconds: number;

            /** Timestamp nanos. */
            public nanos: number;

            /**
             * Creates a new Timestamp instance using the specified properties.
             * @param [properties] Properties to set
             * @returns Timestamp instance
             */
            public static create(properties?: google.protobuf.ITimestamp): google.protobuf.Timestamp;

            /**
             * Encodes the specified Timestamp message. Does not implicitly {@link google.protobuf.Timestamp.verify|verify} messages.
             * @param message Timestamp message or plain object to encode
             * @param [writer] Writer to encode to
             * @returns Writer
             */
            public static encode(message: google.protobuf.ITimestamp, writer?: $protobuf.Writer): $protobuf.Writer;

            /**
             * Encodes the specified Timestamp message, length delimited. Does not implicitly {@link google.protobuf.Timestamp.verify|verify} messages.
             * @param message Timestamp message or plain object to encode
             * @param [writer] Writer to encode to
             * @returns Writer
             */
            public static encodeDelimited(message: google.protobuf.ITimestamp, writer?: $protobuf.Writer): $protobuf.Writer;

            /**
             * Decodes a Timestamp message from the specified reader or buffer.
             * @param reader Reader or buffer to decode from
             * @param [length] Message length if known beforehand
             * @returns Timestamp
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): google.protobuf.Timestamp;

            /**
             * Decodes a Timestamp message from the specified reader or buffer, length delimited.
             * @param reader Reader or buffer to decode from
             * @returns Timestamp
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): google.protobuf.Timestamp;

            /**
             * Verifies a Timestamp message.
             * @param message Plain object to verify
             * @returns `null` if valid, otherwise the reason why it is not
             */
            public static verify(message: { [k: string]: any }): (string|null);

            /**
             * Creates a Timestamp message from a plain object. Also converts values to their respective internal types.
             * @param object Plain object
             * @returns Timestamp
             */
            public static fromObject(object: { [k: string]: any }): google.protobuf.Timestamp;

            /**
             * Creates a plain object from a Timestamp message. Also converts values to other types if specified.
             * @param message Timestamp
             * @param [options] Conversion options
             * @returns Plain object
             */
            public static toObject(message: google.protobuf.Timestamp, options?: $protobuf.IConversionOptions): { [k: string]: any };

            /**
             * Converts this Timestamp to JSON.
             * @returns JSON object
             */
            public toJSON(): { [k: string]: any };
        }

        /** Properties of a Duration. */
        interface IDuration {

            /** Duration seconds */
            seconds?: (number|null);

            /** Duration nanos */
            nanos?: (number|null);
        }

        /** Represents a Duration. */
        class Duration implements IDuration {

            /**
             * Constructs a new Duration.
             * @param [properties] Properties to set
             */
            constructor(properties?: google.protobuf.IDuration);

            /** Duration seconds. */
            public seconds: number;

            /** Duration nanos. */
            public nanos: number;

            /**
             * Creates a new Duration instance using the specified properties.
             * @param [properties] Properties to set
             * @returns Duration instance
             */
            public static create(properties?: google.protobuf.IDuration): google.protobuf.Duration;

            /**
             * Encodes the specified Duration message. Does not implicitly {@link google.protobuf.Duration.verify|verify} messages.
             * @param message Duration message or plain object to encode
             * @param [writer] Writer to encode to
             * @returns Writer
             */
            public static encode(message: google.protobuf.IDuration, writer?: $protobuf.Writer): $protobuf.Writer;

            /**
             * Encodes the specified Duration message, length delimited. Does not implicitly {@link google.protobuf.Duration.verify|verify} messages.
             * @param message Duration message or plain object to encode
             * @param [writer] Writer to encode to
             * @returns Writer
             */
            public static encodeDelimited(message: google.protobuf.IDuration, writer?: $protobuf.Writer): $protobuf.Writer;

            /**
             * Decodes a Duration message from the specified reader or buffer.
             * @param reader Reader or buffer to decode from
             * @param [length] Message length if known beforehand
             * @returns Duration
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): google.protobuf.Duration;

            /**
             * Decodes a Duration message from the specified reader or buffer, length delimited.
             * @param reader Reader or buffer to decode from
             * @returns Duration
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): google.protobuf.Duration;

            /**
             * Verifies a Duration message.
             * @param message Plain object to verify
             * @returns `null` if valid, otherwise the reason why it is not
             */
            public static verify(message: { [k: string]: any }): (string|null);

            /**
             * Creates a Duration message from a plain object. Also converts values to their respective internal types.
             * @param object Plain object
             * @returns Duration
             */
            public static fromObject(object: { [k: string]: any }): google.protobuf.Duration;

            /**
             * Creates a plain object from a Duration message. Also converts values to other types if specified.
             * @param message Duration
             * @param [options] Conversion options
             * @returns Plain object
             */
            public static toObject(message: google.protobuf.Duration, options?: $protobuf.IConversionOptions): { [k: string]: any };

            /**
             * Converts this Duration to JSON.
             * @returns JSON object
             */
            public toJSON(): { [k: string]: any };
        }

        /** Properties of an Any. */
        interface IAny {

            /** Any type_url */
            type_url?: (string|null);

            /** Any value */
            value?: (Uint8Array|null);
        }

        /** Represents an Any. */
        class Any implements IAny {

            /**
             * Constructs a new Any.
             * @param [properties] Properties to set
             */
            constructor(properties?: google.protobuf.IAny);

            /** Any type_url. */
            public type_url: string;

            /** Any value. */
            public value: Uint8Array;

            /**
             * Creates a new Any instance using the specified properties.
             * @param [properties] Properties to set
             * @returns Any instance
             */
            public static create(properties?: google.protobuf.IAny): google.protobuf.Any;

            /**
             * Encodes the specified Any message. Does not implicitly {@link google.protobuf.Any.verify|verify} messages.
             * @param message Any message or plain object to encode
             * @param [writer] Writer to encode to
             * @returns Writer
             */
            public static encode(message: google.protobuf.IAny, writer?: $protobuf.Writer): $protobuf.Writer;

            /**
             * Encodes the specified Any message, length delimited. Does not implicitly {@link google.protobuf.Any.verify|verify} messages.
             * @param message Any message or plain object to encode
             * @param [writer] Writer to encode to
             * @returns Writer
             */
            public static encodeDelimited(message: google.protobuf.IAny, writer?: $protobuf.Writer): $protobuf.Writer;

            /**
             * Decodes an Any message from the specified reader or buffer.
             * @param reader Reader or buffer to decode from
             * @param [length] Message length if known beforehand
             * @returns Any
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): google.protobuf.Any;

            /**
             * Decodes an Any message from the specified reader or buffer, length delimited.
             * @param reader Reader or buffer to decode from
             * @returns Any
             * @throws {Error} If the payload is not a reader or valid buffer
             * @throws {$protobuf.util.ProtocolError} If required fields are missing
             */
            public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): google.protobuf.Any;

            /**
             * Verifies an Any message.
             * @param message Plain object to verify
             * @returns `null` if valid, otherwise the reason why it is not
             */
            public static verify(message: { [k: string]: any }): (string|null);

            /**
             * Creates an Any message from a plain object. Also converts values to their respective internal types.
             * @param object Plain object
             * @returns Any
             */
            public static fromObject(object: { [k: string]: any }): google.protobuf.Any;

            /**
             * Creates a plain object from an Any message. Also converts values to other types if specified.
             * @param message Any
             * @param [options] Conversion options
             * @returns Plain object
             */
            public static toObject(message: google.protobuf.Any, options?: $protobuf.IConversionOptions): { [k: string]: any };

            /**
             * Converts this Any to JSON.
             * @returns JSON object
             */
            public toJSON(): { [k: string]: any };
        }
    }
}
